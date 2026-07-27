//! Hybrid in-memory L1 cache over any offline L2 [`Provider`](super::Provider).

use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use lru::LruCache;

use crate::models::{OfflineCacheError, ProjectLocales, TranslationGroup, TranslationProject};

use super::hybrid_options::{normalize_hybrid_options, HybridOptions};
use super::types::CacheManifest;
use super::{Provider, SaveOptions};

type ProjectCache = ExpiringLru<TranslationProject>;
type GroupCache = ExpiringLru<TranslationGroup>;
type LocalesCache = ExpiringLru<ProjectLocales>;

struct ExpiringLru<V> {
    inner: Mutex<ExpiringLruState<V>>,
    ttl: Duration,
    clock: Arc<dyn Fn() -> Instant + Send + Sync>,
}

struct ExpiringLruState<V> {
    entries: LruCache<String, (Arc<V>, Instant)>,
}

impl<V: Clone> ExpiringLru<V> {
    fn new(max_entries: u32, ttl: Duration) -> Self {
        Self::with_clock(max_entries, ttl, Arc::new(Instant::now))
    }

    fn with_clock(
        max_entries: u32,
        ttl: Duration,
        clock: Arc<dyn Fn() -> Instant + Send + Sync>,
    ) -> Self {
        let capacity = NonZeroUsize::new(max_entries.max(1) as usize).expect("capacity");
        Self {
            inner: Mutex::new(ExpiringLruState {
                entries: LruCache::new(capacity),
            }),
            ttl,
            clock,
        }
    }

    fn get(&self, key: &str) -> Option<Arc<V>> {
        let now = (self.clock)();
        let mut state = self.inner.lock().expect("lock");
        let (value, expires_at) = state.entries.get(key)?;
        if now >= *expires_at {
            state.entries.pop(key);
            return None;
        }
        Some(Arc::clone(value))
    }

    fn insert(&self, key: String, value: Arc<V>) {
        let now = (self.clock)();
        let expires_at = now + self.ttl;
        let mut state = self.inner.lock().expect("lock");
        state.entries.put(key, (value, expires_at));
    }

    fn contains(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn purge(&self) {
        let mut state = self.inner.lock().expect("lock");
        state.entries.clear();
    }

    fn len(&self) -> usize {
        let now = (self.clock)();
        let mut state = self.inner.lock().expect("lock");
        let expired: Vec<String> = state
            .entries
            .iter()
            .filter(|(_, (_, expires_at))| now >= *expires_at)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            state.entries.pop(&key);
        }
        state.entries.len()
    }
}

struct HybridL1 {
    projects: ProjectCache,
    groups: GroupCache,
    locales: LocalesCache,
}

impl HybridL1 {
    fn new(max_entries: u32, ttl: Duration) -> Self {
        Self {
            projects: ExpiringLru::new(max_entries, ttl),
            groups: ExpiringLru::new(max_entries, ttl),
            locales: ExpiringLru::new(max_entries, ttl),
        }
    }

    fn purge(&self) {
        self.projects.purge();
        self.groups.purge();
        self.locales.purge();
    }

    fn stats(&self) -> (usize, usize, usize) {
        (self.projects.len(), self.groups.len(), self.locales.len())
    }
}

/// Combines an expirable LRU memory cache (L1) with any disk [`Provider`] (L2).
///
/// L1 uses strict LRU eviction with per-entry TTL, aligned with Go's
/// `hashicorp/golang-lru/v2/expirable`. We evaluated **`moka`** (TTL + concurrent
/// cache) and **`quick_cache`** (LRU); `moka`'s W-TinyLFU admission policy
/// differs from Go on capacity eviction, and `quick_cache` has no cache-level TTL,
/// so this module uses the [`lru`](https://docs.rs/lru) crate with explicit expiry.
///
/// This is separate from HTTP [`crate::cache::MemoryProvider`].
pub struct HybridProvider<L2> {
    l2: L2,
    #[allow(dead_code)]
    opts: HybridOptions,
    l1: Option<HybridL1>,
}

impl<L2: Provider> HybridProvider<L2> {
    /// Wraps `l2` with an optional in-memory L1 cache according to `options`.
    pub fn new(l2: L2, options: HybridOptions) -> Self {
        let opts = normalize_hybrid_options(options);
        let l1 = if opts.enabled {
            Some(HybridL1::new(opts.max_entries, opts.memory_expiration))
        } else {
            None
        };
        Self { l2, opts, l1 }
    }

    /// Removes all L1 entries without touching L2.
    pub fn clear_memory_cache(&self) {
        if let Some(l1) = &self.l1 {
            l1.purge();
        }
    }

    /// Returns current L1 entry counts: `(projects, groups, locales)`.
    pub fn memory_cache_stats(&self) -> (usize, usize, usize) {
        self.l1.as_ref().map(HybridL1::stats).unwrap_or((0, 0, 0))
    }

    /// Loads project data from L2 into L1. Returns `Ok(false)` on L2 miss.
    pub fn warmup(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError> {
        let Some(l1) = &self.l1 else {
            return Ok(false);
        };

        let data = self.l2.get_project(project, lang)?;
        let Some(data) = data else {
            return Ok(false);
        };

        let data = Arc::new(data);
        l1.projects
            .insert(project_cache_key(project, lang), Arc::clone(&data));
        cache_project_groups_l1(l1, project, lang, &data);
        Ok(true)
    }

    fn cache_project_groups_l1(&self, project: &str, lang: &str, data: &TranslationProject) {
        if let Some(l1) = &self.l1 {
            cache_project_groups_l1(l1, project, lang, data);
        }
    }
}

impl HybridProvider<Arc<dyn Provider>> {
    /// Creates a hybrid provider from an optional L2 handle (for dynamic wiring).
    ///
    /// Returns an error when `l2` is `None`.
    pub fn try_new(
        l2: Option<Arc<dyn Provider>>,
        options: HybridOptions,
    ) -> Result<Self, OfflineCacheError> {
        let l2 = l2.ok_or_else(|| {
            OfflineCacheError::new(
                "cachefile: L2 provider must not be nil",
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(Self::new(l2, options))
    }
}

impl<L2: Provider> Provider for HybridProvider<L2> {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            if let Some(cached) = l1.projects.get(&project_cache_key(project, lang)) {
                return Ok(Some((*cached).clone()));
            }
        }

        let result = self.l2.get_project(project, lang)?;
        if let (Some(l1), Some(data)) = (&self.l1, &result) {
            l1.projects
                .insert(project_cache_key(project, lang), Arc::new(data.clone()));
        }
        Ok(result)
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            l1.projects
                .insert(project_cache_key(project, lang), Arc::new(data.clone()));
            self.cache_project_groups_l1(project, lang, data);
        }
        self.l2.save_project(project, lang, data, options)
    }

    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            if let Some(cached) = l1.groups.get(&group_cache_key(project, group, lang)) {
                return Ok(Some((*cached).clone()));
            }
        }

        let result = self.l2.get_group(project, group, lang)?;
        if let (Some(l1), Some(data)) = (&self.l1, &result) {
            l1.groups.insert(
                group_cache_key(project, group, lang),
                Arc::new(data.clone()),
            );
        }
        Ok(result)
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            if let Some(cached) = l1.locales.get(&locales_cache_key(project)) {
                return Ok(Some((*cached).clone()));
            }
        }

        let result = self.l2.get_locales(project)?;
        if let (Some(l1), Some(data)) = (&self.l1, &result) {
            l1.locales
                .insert(locales_cache_key(project), Arc::new(data.clone()));
        }
        Ok(result)
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            l1.locales
                .insert(locales_cache_key(project), Arc::new(data.clone()));
        }
        self.l2.save_locales(project, data, options)
    }

    fn get_manifest(&self) -> Result<Option<CacheManifest>, OfflineCacheError> {
        self.l2.get_manifest()
    }

    fn update_manifest(
        &self,
        update: &mut dyn FnMut(&mut CacheManifest) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        self.l2.update_manifest(update)
    }

    fn is_cached(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError> {
        if let Some(l1) = &self.l1 {
            if l1.projects.contains(&project_cache_key(project, lang)) {
                return Ok(true);
            }
        }
        self.l2.is_cached(project, lang)
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        self.clear_memory_cache();
        self.l2.clear()
    }
}

fn cache_project_groups_l1(l1: &HybridL1, project: &str, lang: &str, data: &TranslationProject) {
    for name in data.groups.keys() {
        let Ok(Some(group)) = data.get_group(name) else {
            continue;
        };
        l1.groups
            .insert(group_cache_key(project, name, lang), Arc::new(group));
    }
}

fn project_cache_key(project: &str, lang: &str) -> String {
    format!("project:{project}:{lang}")
}

fn group_cache_key(project: &str, group: &str, lang: &str) -> String {
    format!("group:{project}:{group}:{lang}")
}

fn locales_cache_key(project: &str) -> String {
    format!("locales:{project}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn expiring_lru_evicts_oldest_at_capacity() {
        let start = Instant::now();
        let offset = Arc::new(AtomicU64::new(0));
        let clock = {
            let offset = Arc::clone(&offset);
            Arc::new(move || start + Duration::from_millis(offset.load(Ordering::SeqCst)))
        };
        let cache = ExpiringLru::<String>::with_clock(2, Duration::from_secs(60), clock);

        cache.insert("a".to_string(), Arc::new("A".to_string()));
        cache.insert("b".to_string(), Arc::new("B".to_string()));
        cache.insert("c".to_string(), Arc::new("C".to_string()));

        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }
}
