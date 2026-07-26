//! Thread-safe in-memory cache provider.

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::memory_options::{MemoryOptions, Statistics};
use super::{CacheError, Provider, Ttl};

struct CacheEntry {
    value: Box<dyn Any + Send + Sync>,
    absolute_expiry: Option<Instant>,
    sliding: Option<Duration>,
    last_access: Instant,
}

impl CacheEntry {
    fn expired(&self, now: Instant) -> bool {
        if let Some(expiry) = self.absolute_expiry {
            if now >= expiry {
                return true;
            }
        }
        if let Some(sliding) = self.sliding {
            if now.duration_since(self.last_access) >= sliding {
                return true;
            }
        }
        false
    }
}

/// Thread-safe in-memory [`Provider`] with optional TTL, LRU eviction, and stats.
pub struct MemoryProvider {
    inner: Mutex<MemoryState>,
    max_size: usize,
    enable_statistics: bool,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

struct MemoryState {
    entries: HashMap<String, CacheEntry>,
    hits: u64,
    misses: u64,
}

impl Default for MemoryProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProvider {
    /// Creates a provider with default options.
    pub fn new() -> Self {
        Self::with_options(MemoryOptions::default())
    }

    /// Creates a provider with the given options.
    pub fn with_options(options: MemoryOptions) -> Self {
        Self {
            inner: Mutex::new(MemoryState {
                entries: HashMap::new(),
                hits: 0,
                misses: 0,
            }),
            max_size: options.max_size.unwrap_or(0),
            enable_statistics: options.enable_statistics,
            clock: options.clock.unwrap_or_else(|| Arc::new(Instant::now)),
        }
    }

    /// Returns cache counters when statistics are enabled.
    pub fn statistics(&self) -> Option<Statistics> {
        if !self.enable_statistics {
            return None;
        }
        let state = self.inner.lock().expect("memory cache mutex poisoned");
        Some(Statistics {
            hits: state.hits,
            misses: state.misses,
            size: state.entries.len(),
        })
    }

    fn now(&self) -> Instant {
        (self.clock)()
    }
}

impl Provider for MemoryProvider {
    fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let mut state = self.inner.lock().expect("memory cache mutex poisoned");
        let now = self.now();

        let Some(entry) = state.entries.get_mut(key) else {
            record_miss(&mut state, self.enable_statistics);
            return Ok(None);
        };

        if entry.expired(now) {
            state.entries.remove(key);
            record_miss(&mut state, self.enable_statistics);
            return Ok(None);
        }

        entry.last_access = now;
        let value = clone_any::<T>(&entry.value)?;
        record_hit(&mut state, self.enable_statistics);
        Ok(Some(value))
    }

    fn set<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        value: T,
        ttl: Ttl,
    ) -> Result<(), CacheError> {
        let mut state = self.inner.lock().expect("memory cache mutex poisoned");
        let now = self.now();

        let absolute_expiry = ttl.absolute.filter(|d| !d.is_zero()).map(|d| now + d);

        let sliding = ttl.sliding.filter(|d| !d.is_zero());

        let entry = CacheEntry {
            value: Box::new(value),
            absolute_expiry,
            sliding,
            last_access: now,
        };

        if state.entries.contains_key(key) {
            state.entries.insert(key.to_string(), entry);
            return Ok(());
        }

        if self.max_size > 0 && state.entries.len() >= self.max_size {
            evict_lru(&mut state.entries, now);
        }

        state.entries.insert(key.to_string(), entry);
        Ok(())
    }

    fn remove(&self, key: &str) -> Result<(), CacheError> {
        let mut state = self.inner.lock().expect("memory cache mutex poisoned");
        state.entries.remove(key);
        Ok(())
    }

    fn clear(&self) -> Result<(), CacheError> {
        let mut state = self.inner.lock().expect("memory cache mutex poisoned");
        state.entries.clear();
        if self.enable_statistics {
            state.hits = 0;
            state.misses = 0;
        }
        Ok(())
    }
}

fn clone_any<T: Clone + Send + Sync + 'static>(
    value: &Box<dyn Any + Send + Sync>,
) -> Result<T, CacheError> {
    value
        .downcast_ref::<T>()
        .cloned()
        .ok_or(CacheError::TypeMismatch)
}

fn record_hit(state: &mut MemoryState, enable_statistics: bool) {
    if enable_statistics {
        state.hits += 1;
    }
}

fn record_miss(state: &mut MemoryState, enable_statistics: bool) {
    if enable_statistics {
        state.misses += 1;
    }
}

fn evict_lru(entries: &mut HashMap<String, CacheEntry>, _now: Instant) {
    if entries.is_empty() {
        return;
    }

    let mut oldest_key: Option<String> = None;
    let mut oldest_access = Instant::now();

    for (key, entry) in entries.iter() {
        if oldest_key.is_none() || entry.last_access < oldest_access {
            oldest_key = Some(key.clone());
            oldest_access = entry.last_access;
        }
    }

    if let Some(key) = oldest_key {
        entries.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::models::TranslationGroup;

    struct TestClock {
        now: Arc<Mutex<Instant>>,
    }

    impl TestClock {
        fn new(base: Instant) -> Self {
            Self {
                now: Arc::new(Mutex::new(base)),
            }
        }

        fn advance(&self, duration: Duration) {
            let mut now = self.now.lock().expect("test clock mutex poisoned");
            *now += duration;
        }

        fn handle(&self) -> Arc<dyn Fn() -> Instant + Send + Sync> {
            let now = Arc::clone(&self.now);
            Arc::new(move || *now.lock().expect("test clock mutex poisoned"))
        }
    }

    fn new_test_provider(options: MemoryOptions) -> (MemoryProvider, Arc<TestClock>) {
        let base = Instant::now();
        let clock = Arc::new(TestClock::new(base));
        let provider = MemoryProvider::with_options(options.with_clock(clock.handle()));
        (provider, clock)
    }

    #[test]
    fn set_and_get_string() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        provider
            .set("k1", "value1".to_string(), Ttl::none())
            .expect("set");
        let got: Option<String> = provider.get("k1").expect("get");
        assert_eq!(got.as_deref(), Some("value1"));
    }

    #[test]
    fn get_translation_group() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        let group = TranslationGroup {
            project: Some("p".to_string()),
            lang: Some("en".to_string()),
            ..Default::default()
        };
        provider
            .set("group", group.clone(), Ttl::none())
            .expect("set");
        let got: Option<TranslationGroup> = provider.get("group").expect("get");
        assert_eq!(got.as_ref().map(|g| g.project.as_deref()), Some(Some("p")));
    }

    #[test]
    fn miss_returns_none() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        let got: Option<String> = provider.get("missing").expect("get");
        assert!(got.is_none());
    }

    #[test]
    fn absolute_expiration() {
        let (provider, clock) = new_test_provider(MemoryOptions::default());
        provider
            .set(
                "k1",
                "value1".to_string(),
                Ttl::absolute(Duration::from_millis(100)),
            )
            .expect("set");

        let got: Option<String> = provider.get("k1").expect("get");
        assert_eq!(got.as_deref(), Some("value1"));

        clock.advance(Duration::from_millis(150));
        let got: Option<String> = provider.get("k1").expect("get");
        assert!(got.is_none());
    }

    #[test]
    fn sliding_expiration() {
        let (provider, clock) = new_test_provider(MemoryOptions::default());
        provider
            .set(
                "k1",
                "value1".to_string(),
                Ttl::sliding(Duration::from_millis(200)),
            )
            .expect("set");

        let _: Option<String> = provider.get("k1").expect("get");
        clock.advance(Duration::from_millis(100));
        let _: Option<String> = provider.get("k1").expect("get");

        clock.advance(Duration::from_millis(250));
        let got: Option<String> = provider.get("k1").expect("get");
        assert!(got.is_none());
    }

    #[test]
    fn both_expirations_absolute_wins() {
        let (provider, clock) = new_test_provider(MemoryOptions::default());
        provider
            .set(
                "k1",
                "value1".to_string(),
                Ttl::both(Duration::from_millis(100), Duration::from_secs(1)),
            )
            .expect("set");

        clock.advance(Duration::from_millis(150));
        let got: Option<String> = provider.get("k1").expect("get");
        assert!(got.is_none());
    }

    #[test]
    fn remove_and_clear() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        provider
            .set("k1", "v1".to_string(), Ttl::none())
            .expect("set");
        provider
            .set("k2", "v2".to_string(), Ttl::none())
            .expect("set");

        provider.remove("k1").expect("remove");
        let got: Option<String> = provider.get("k1").expect("get");
        assert!(got.is_none());

        provider.clear().expect("clear");
        let got: Option<String> = provider.get("k2").expect("get");
        assert!(got.is_none());
    }

    #[test]
    fn lru_eviction() {
        let (provider, clock) = new_test_provider(MemoryOptions::default().with_max_size(2));
        provider
            .set("k1", "v1".to_string(), Ttl::none())
            .expect("set");
        clock.advance(Duration::from_millis(1));
        provider
            .set("k2", "v2".to_string(), Ttl::none())
            .expect("set");
        clock.advance(Duration::from_millis(1));

        let _: Option<String> = provider.get("k1").expect("get");
        provider
            .set("k3", "v3".to_string(), Ttl::none())
            .expect("set");

        let k2: Option<String> = provider.get("k2").expect("get");
        assert!(k2.is_none());

        let k1: Option<String> = provider.get("k1").expect("get");
        assert_eq!(k1.as_deref(), Some("v1"));

        let k3: Option<String> = provider.get("k3").expect("get");
        assert_eq!(k3.as_deref(), Some("v3"));
    }

    #[test]
    fn statistics() {
        let (provider, _) = new_test_provider(MemoryOptions::default().with_statistics());
        provider
            .set("k1", "v1".to_string(), Ttl::none())
            .expect("set");
        let _: Option<String> = provider.get("k1").expect("get");
        let _: Option<String> = provider.get("missing").expect("get");

        let stats = provider.statistics().expect("stats enabled");
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.size, 1);

        provider.clear().expect("clear");
        let stats = provider.statistics().expect("stats enabled");
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn concurrent_access() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        let provider = Arc::new(provider);

        let mut handles = Vec::new();
        for _ in 0..50 {
            let provider = Arc::clone(&provider);
            handles.push(thread::spawn(move || {
                provider
                    .set("key", "value".to_string(), Ttl::none())
                    .expect("set");
                let got: Option<String> = provider.get("key").expect("get");
                assert!(got.is_some());
            }));
        }

        for handle in handles {
            handle.join().expect("thread join");
        }
    }

    #[test]
    fn type_mismatch() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        provider
            .set("k", "string-value".to_string(), Ttl::none())
            .expect("set");
        let got: Result<Option<i32>, _> = provider.get("k");
        assert_eq!(got, Err(CacheError::TypeMismatch));
    }

    #[test]
    fn set_overwrite() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        provider
            .set("k", "v1".to_string(), Ttl::none())
            .expect("set");
        provider
            .set("k", "v2".to_string(), Ttl::none())
            .expect("set");
        let got: Option<String> = provider.get("k").expect("get");
        assert_eq!(got.as_deref(), Some("v2"));
    }

    #[test]
    fn remove_missing_key_is_ok() {
        let (provider, _) = new_test_provider(MemoryOptions::default());
        provider.remove("missing").expect("remove");
    }
}
