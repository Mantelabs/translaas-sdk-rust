//! Integration tests for `translaas::cachefile::HybridProvider`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::json;
use translaas::cachefile::{
    FileProvider, HybridOptions, HybridProvider, Provider, SaveOptions,
};
use translaas::models::{OfflineCacheError, ProjectLocales, TranslationGroup, TranslationProject};

struct MockL2Provider {
    inner: Mutex<MockL2State>,
    get_project_calls: AtomicUsize,
    get_group_calls: AtomicUsize,
    is_cached_calls: AtomicUsize,
}

struct MockL2State {
    projects: HashMap<String, TranslationProject>,
    groups: HashMap<String, TranslationGroup>,
    locales: HashMap<String, ProjectLocales>,
    cached: HashMap<String, bool>,
}

impl MockL2Provider {
    fn new() -> Self {
        Self {
            inner: Mutex::new(MockL2State {
                projects: HashMap::new(),
                groups: HashMap::new(),
                locales: HashMap::new(),
                cached: HashMap::new(),
            }),
            get_project_calls: AtomicUsize::new(0),
            get_group_calls: AtomicUsize::new(0),
            is_cached_calls: AtomicUsize::new(0),
        }
    }

    fn get_project_call_count(&self) -> usize {
        self.get_project_calls.load(Ordering::SeqCst)
    }

    fn get_group_call_count(&self) -> usize {
        self.get_group_calls.load(Ordering::SeqCst)
    }

    fn is_cached_call_count(&self) -> usize {
        self.is_cached_calls.load(Ordering::SeqCst)
    }
}

impl Provider for MockL2Provider {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        self.get_project_calls.fetch_add(1, Ordering::SeqCst);
        let state = self.inner.lock().expect("lock");
        Ok(state.projects.get(&format!("{project}:{lang}")).cloned())
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        _options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().expect("lock");
        state
            .projects
            .insert(format!("{project}:{lang}"), data.clone());
        state.cached.insert(format!("{project}:{lang}"), true);
        Ok(())
    }

    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        self.get_group_calls.fetch_add(1, Ordering::SeqCst);
        let state = self.inner.lock().expect("lock");
        Ok(state
            .groups
            .get(&format!("{project}:{group}:{lang}"))
            .cloned())
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        let state = self.inner.lock().expect("lock");
        Ok(state.locales.get(project).cloned())
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        _options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().expect("lock");
        state.locales.insert(project.to_string(), data.clone());
        Ok(())
    }

    fn get_manifest(
        &self,
    ) -> Result<Option<translaas::cachefile::CacheManifest>, OfflineCacheError> {
        Ok(None)
    }

    fn update_manifest(
        &self,
        _update: &mut dyn FnMut(
            &mut translaas::cachefile::CacheManifest,
        ) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        Ok(())
    }

    fn is_cached(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError> {
        self.is_cached_calls.fetch_add(1, Ordering::SeqCst);
        let state = self.inner.lock().expect("lock");
        Ok(state
            .cached
            .get(&format!("{project}:{lang}"))
            .copied()
            .unwrap_or(false))
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().expect("lock");
        state.projects.clear();
        state.groups.clear();
        state.locales.clear();
        state.cached.clear();
        Ok(())
    }
}

fn test_project(label: &str) -> TranslationProject {
    let mut groups = HashMap::new();
    groups.insert("common".to_string(), json!({ label: label }));
    TranslationProject {
        groups,
        ..Default::default()
    }
}

fn seed_l2_project(l2: &MockL2Provider, project: &str, lang: &str, label: &str) {
    l2.inner
        .lock()
        .expect("lock")
        .projects
        .insert(format!("{project}:{lang}"), test_project(label));
}

fn new_hybrid_with_mock_arc(
    opts: HybridOptions,
) -> (HybridProvider<Arc<MockL2Provider>>, Arc<MockL2Provider>) {
    let l2 = Arc::new(MockL2Provider::new());
    let provider = HybridProvider::new(Arc::clone(&l2), opts);
    (provider, l2)
}

#[test]
fn new_hybrid_provider_requires_l2() {
    match HybridProvider::<Arc<dyn Provider>>::try_new(None, HybridOptions::default()) {
        Err(err) => assert!(err.message.contains("L2 provider must not be nil")),
        Ok(_) => panic!("expected error for nil L2"),
    }
}

#[test]
fn hybrid_provider_promotes_l2_hit_to_l1() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "demo", "en", "Hello");

    let provider = HybridProvider::new(Arc::clone(&l2), HybridOptions::default());

    provider
        .get_project("demo", "en")
        .expect("first get")
        .expect("project");
    provider
        .get_project("demo", "en")
        .expect("second get")
        .expect("project");

    assert_eq!(l2.get_project_call_count(), 1);
}

#[test]
fn hybrid_provider_save_project_populates_l1() {
    let (provider, l2) = new_hybrid_with_mock_arc(HybridOptions::default());
    let project = test_project("saved");

    provider
        .save_project("demo", "en", &project, SaveOptions::new())
        .expect("save");

    provider
        .get_project("demo", "en")
        .expect("get project")
        .expect("cached");
    assert_eq!(l2.get_project_call_count(), 0);

    provider
        .get_group("demo", "common", "en")
        .expect("get group")
        .expect("group");
    assert_eq!(l2.get_group_call_count(), 0);
}

#[test]
fn hybrid_provider_l1_expires_after_ttl() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "demo", "en", "Hello");

    let opts = HybridOptions::default().with_memory_expiration(Duration::from_millis(50));
    let provider = HybridProvider::new(Arc::clone(&l2), opts);

    provider
        .get_project("demo", "en")
        .expect("first get")
        .expect("project");

    thread::sleep(Duration::from_millis(100));

    provider
        .get_project("demo", "en")
        .expect("second get")
        .expect("project");

    assert_eq!(l2.get_project_call_count(), 2);
}

#[test]
fn hybrid_provider_lru_evicts_oldest_entry() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "a", "en", "A");
    seed_l2_project(&l2, "b", "en", "B");
    seed_l2_project(&l2, "c", "en", "C");

    let opts = HybridOptions::default()
        .with_max_entries(2)
        .with_memory_expiration(Duration::from_secs(60));
    let provider = HybridProvider::new(Arc::clone(&l2), opts);

    for id in ["a", "b", "c"] {
        provider
            .get_project(id, "en")
            .expect("get")
            .expect("project");
    }

    provider
        .get_project("a", "en")
        .expect("get a after eviction")
        .expect("project");

    assert_eq!(l2.get_project_call_count(), 4);
}

#[test]
fn hybrid_provider_is_cached_uses_l1() {
    let (provider, l2) = new_hybrid_with_mock_arc(HybridOptions::default());

    provider
        .save_project("demo", "en", &test_project("x"), SaveOptions::new())
        .expect("save");

    let cached = provider.is_cached("demo", "en").expect("is_cached");
    assert!(cached);
    assert_eq!(l2.is_cached_call_count(), 0);
}

#[test]
fn hybrid_provider_clear_removes_l1_and_l2() {
    let (provider, _l2) = new_hybrid_with_mock_arc(HybridOptions::default());

    provider
        .save_project("demo", "en", &test_project("x"), SaveOptions::new())
        .expect("save");

    provider.clear().expect("clear");

    let (projects, groups, locales) = provider.memory_cache_stats();
    assert_eq!((projects, groups, locales), (0, 0, 0));
}

#[test]
fn hybrid_provider_warmup_populates_l1() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "demo", "en", "warm");

    let provider = HybridProvider::new(Arc::clone(&l2), HybridOptions::default());

    let warmed = provider.warmup("demo", "en").expect("warmup");
    assert!(warmed);

    provider
        .get_project("demo", "en")
        .expect("get")
        .expect("project");
    assert_eq!(l2.get_project_call_count(), 1);
}

#[test]
fn hybrid_provider_disabled_delegates_to_l2_only() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "demo", "en", "delegated");

    let opts = HybridOptions::default().disabled();
    let provider = HybridProvider::new(Arc::clone(&l2), opts);

    provider
        .get_project("demo", "en")
        .expect("first get")
        .expect("project");
    provider
        .get_project("demo", "en")
        .expect("second get")
        .expect("project");

    assert_eq!(l2.get_project_call_count(), 2);
}

#[test]
fn hybrid_provider_integration_with_file_provider() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_provider = FileProvider::new(dir.path()).expect("file provider");
    let provider = HybridProvider::new(file_provider, HybridOptions::default());

    provider
        .save_project("demo", "en", &test_project("integration"), SaveOptions::new())
        .expect("save");

    let got = provider
        .get_project("demo", "en")
        .expect("get")
        .expect("project");
    assert!(got.groups.contains_key("common"));

    provider.clear_memory_cache();

    let got = provider
        .get_project("demo", "en")
        .expect("get after clear memory")
        .expect("project");
    assert!(got.groups.contains_key("common"));
}

#[test]
fn hybrid_provider_concurrent_access_race_safe() {
    let l2 = Arc::new(MockL2Provider::new());
    seed_l2_project(&l2, "demo", "en", "race");

    let provider = Arc::new(HybridProvider::new(
        Arc::clone(&l2),
        HybridOptions::default(),
    ));

    let mut handles = Vec::new();
    for _ in 0..16 {
        let provider = Arc::clone(&provider);
        handles.push(thread::spawn(move || {
            let _ = provider.get_project("demo", "en");
        }));
    }
    for handle in handles {
        handle.join().expect("thread join");
    }
}
