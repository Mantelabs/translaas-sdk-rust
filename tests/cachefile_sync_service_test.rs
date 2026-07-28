//! Integration tests for `cachefile::SyncService`.

#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio_util::sync::CancellationToken;
use translaas::cachefile::{
    CachingClient, CachingOptions, FallbackMode, OfflineCacheOptions, Provider, SaveOptions,
    SyncCallbacks, SyncCompletedEvent, SyncResult, SyncService,
};
use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    ConfigurationError, OfflineCacheDownloadResult, OfflineCacheError, ProjectLocales,
    ReportMissingKeyItem, TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};

struct SyncMockClient {
    state: Mutex<SyncMockClientState>,
    get_project_fn: Option<
        Box<
            dyn Fn(
                    &str,
                    &str,
                    GetProjectOptions<'_>,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<TranslationProject, Error>> + Send>,
                > + Send
                + Sync,
        >,
    >,
    get_project_locales_fn: Option<
        Box<
            dyn Fn(
                    &str,
                    GetProjectLocalesOptions<'_>,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<ProjectLocales, Error>> + Send>,
                > + Send
                + Sync,
        >,
    >,
}

#[derive(Default)]
struct SyncMockClientState {
    get_project_calls: u32,
    get_project_locales_calls: u32,
}

impl SyncMockClient {
    fn new() -> Self {
        Self {
            state: Mutex::new(SyncMockClientState::default()),
            get_project_fn: None,
            get_project_locales_fn: None,
        }
    }

    fn get_project_call_count(&self) -> u32 {
        self.state.lock().unwrap().get_project_calls
    }
}

impl TranslaasClient for SyncMockClient {
    async fn get_entry(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        Err(unexpected("get_entry"))
    }

    async fn get_group(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        Err(unexpected("get_group"))
    }

    async fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        self.state.lock().unwrap().get_project_calls += 1;
        if let Some(ref handler) = self.get_project_fn {
            return handler(project, lang, opts).await;
        }
        Err(unexpected("get_project"))
    }

    async fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        self.state.lock().unwrap().get_project_locales_calls += 1;
        if let Some(ref handler) = self.get_project_locales_fn {
            return handler(project, opts).await;
        }
        Err(unexpected("get_project_locales"))
    }

    async fn get_offline_cache(
        &self,
        _: &str,
        _: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        Err(unexpected("get_offline_cache"))
    }

    async fn report_missing_keys(&self, _: &[ReportMissingKeyItem]) -> Result<(), Error> {
        Err(unexpected("report_missing_keys"))
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        Err(unexpected("validate_api_key"))
    }
}

fn unexpected(method: &str) -> Error {
    Error::Configuration(ConfigurationError {
        message: format!("unexpected {method}"),
    })
}

struct SyncMockCache {
    inner: Mutex<SyncMockCacheState>,
}

#[derive(Default)]
struct SyncMockCacheState {
    save_project_calls: u32,
    save_locales_calls: u32,
    projects: HashMap<String, TranslationProject>,
    locales: HashMap<String, ProjectLocales>,
}

impl SyncMockCache {
    fn new() -> Self {
        Self {
            inner: Mutex::new(SyncMockCacheState::default()),
        }
    }

    fn save_project_call_count(&self) -> u32 {
        self.inner.lock().unwrap().save_project_calls
    }

    fn save_locales_call_count(&self) -> u32 {
        self.inner.lock().unwrap().save_locales_calls
    }
}

impl Provider for SyncMockCache {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        Ok(self
            .inner
            .lock()
            .unwrap()
            .projects
            .get(&format!("{project}:{lang}"))
            .cloned())
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        _: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().unwrap();
        state.save_project_calls += 1;
        state
            .projects
            .insert(format!("{project}:{lang}"), data.clone());
        Ok(())
    }

    fn get_group(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        Ok(None)
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        Ok(self.inner.lock().unwrap().locales.get(project).cloned())
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        _: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().unwrap();
        state.save_locales_calls += 1;
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
        _: &mut dyn FnMut(
            &mut translaas::cachefile::CacheManifest,
        ) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        Ok(())
    }

    fn is_cached(&self, _: &str, _: &str) -> Result<bool, OfflineCacheError> {
        Ok(false)
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        Ok(())
    }
}

fn test_project(lang: &str) -> TranslationProject {
    TranslationProject {
        groups: HashMap::from([(
            "common".to_string(),
            serde_json::json!({ "hello": format!("Hello {lang}") }),
        )]),
        ..Default::default()
    }
}

fn demo_locales(project: &str, locales: Vec<String>) -> ProjectLocales {
    ProjectLocales {
        project: Some(project.to_string()),
        locales,
        last_modified_utc: None,
    }
}

/// Wrapper so the same mock inner client can be shared between sync and decorator tests.
#[derive(Clone)]
struct SharedSyncMockClient(Arc<SyncMockClient>);

impl TranslaasClient for SharedSyncMockClient {
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        self.0.get_entry(group, entry, lang, opts).await
    }

    async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        self.0.get_group(project, group, lang, opts).await
    }

    async fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        self.0.get_project(project, lang, opts).await
    }

    async fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        self.0.get_project_locales(project, opts).await
    }

    async fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        self.0.get_offline_cache(project, opts).await
    }

    async fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        self.0.report_missing_keys(keys).await
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        self.0.validate_api_key().await
    }
}

fn new_sync_service(
    inner: SyncMockClient,
    cache: SyncMockCache,
    opts: OfflineCacheOptions,
    callbacks: SyncCallbacks,
) -> SyncService<SyncMockClient, SyncMockCache> {
    SyncService::new(inner, cache, opts, callbacks)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_fetches_and_caches() {
    let inner = SyncMockClient {
        get_project_fn: Some(Box::new(|project, lang, _| {
            let project = project.to_string();
            let lang = lang.to_string();
            Box::pin(async move {
                if project != "demo" || lang != "en" {
                    panic!("unexpected GetProject({project}, {lang})");
                }
                Ok(test_project("en"))
            })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let svc = new_sync_service(
        inner,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_project("demo", "en", &cancel)
        .await
        .expect("sync_project");

    assert_eq!(svc.cache().save_project_call_count(), 1);
    assert_eq!(svc.client().get_project_call_count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_invokes_on_sync_completed() {
    let inner = SyncMockClient {
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(TranslationProject::default()) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let completed = Arc::new(Mutex::new(None::<SyncCompletedEvent>));
    let completed_cb = Arc::clone(&completed);

    let callbacks = SyncCallbacks {
        on_sync_completed: Some(Arc::new(move |event| {
            *completed_cb.lock().unwrap() = Some(event);
        })),
        ..SyncCallbacks::default()
    };

    let svc = new_sync_service(
        inner,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        callbacks,
    );
    let cancel = CancellationToken::new();

    svc.sync_project("demo", "en", &cancel)
        .await
        .expect("sync_project");

    let event = completed.lock().unwrap().clone().expect("completed event");
    assert_eq!(event.project, "demo");
    assert_eq!(event.language, "en");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_invokes_on_sync_failed_and_returns_error() {
    let inner = SyncMockClient {
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async {
                Err(Error::Configuration(ConfigurationError {
                    message: "api down".to_string(),
                }))
            })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let failed_project = Arc::new(Mutex::new(None::<String>));
    let failed_project_cb = Arc::clone(&failed_project);

    let callbacks = SyncCallbacks {
        on_sync_failed: Some(Arc::new(move |event| {
            *failed_project_cb.lock().unwrap() = Some(event.project);
        })),
        ..SyncCallbacks::default()
    };

    let svc = new_sync_service(
        inner,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        callbacks,
    );
    let cancel = CancellationToken::new();

    let err = svc
        .sync_project("demo", "en", &cancel)
        .await
        .expect_err("expected error");
    assert!(matches!(err, Error::Configuration(_)));
    assert_eq!(failed_project.lock().unwrap().as_deref(), Some("demo"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_all_languages_respects_language_filter() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move {
                Ok(demo_locales(
                    &project,
                    vec!["en".into(), "es".into(), "fr".into()],
                ))
            })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.languages = vec!["en".into(), "es".into()];
    let svc = new_sync_service(inner, cache, opts, SyncCallbacks::default());
    let cancel = CancellationToken::new();

    svc.sync_project_all_languages("demo", &cancel)
        .await
        .expect("sync all langs");

    assert_eq!(svc.cache().save_locales_call_count(), 1);
    assert_eq!(svc.client().get_project_call_count(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_all_languages_syncs_all_when_filter_empty() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move {
                Ok(demo_locales(
                    &project,
                    vec!["en".into(), "es".into(), "fr".into()],
                ))
            })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let svc = new_sync_service(
        inner,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_project_all_languages("demo", &cancel)
        .await
        .expect("sync all langs");

    assert_eq!(svc.client().get_project_call_count(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_all_languages_continues_on_language_failure() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move { Ok(demo_locales(&project, vec!["en".into(), "es".into()])) })
        })),
        get_project_fn: Some(Box::new(|_, lang, _| {
            let lang = lang.to_string();
            Box::pin(async move {
                if lang == "en" {
                    Err(Error::Configuration(ConfigurationError {
                        message: "en failed".to_string(),
                    }))
                } else {
                    Ok(test_project(&lang))
                }
            })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let failed_langs = Arc::new(Mutex::new(Vec::<String>::new()));
    let failed_langs_cb = Arc::clone(&failed_langs);

    let callbacks = SyncCallbacks {
        on_sync_failed: Some(Arc::new(move |event| {
            failed_langs_cb.lock().unwrap().push(event.language);
        })),
        ..SyncCallbacks::default()
    };

    let svc = new_sync_service(
        inner,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        callbacks,
    );
    let cancel = CancellationToken::new();

    svc.sync_project_all_languages("demo", &cancel)
        .await
        .expect("continues on failure");

    assert_eq!(*failed_langs.lock().unwrap(), vec!["en".to_string()]);
    assert_eq!(svc.cache().save_project_call_count(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_all_aggregates_partial_failures() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move {
                if project == "bad" {
                    Err(Error::Configuration(ConfigurationError {
                        message: "locales failed".to_string(),
                    }))
                } else {
                    Ok(demo_locales(&project, vec!["en".into()]))
                }
            })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.projects = vec!["bad".into(), "good".into()];

    let all_completed = Arc::new(Mutex::new(None::<SyncResult>));
    let all_completed_cb = Arc::clone(&all_completed);

    let callbacks = SyncCallbacks {
        on_sync_all_completed: Some(Arc::new(move |result| {
            *all_completed_cb.lock().unwrap() = Some(result);
        })),
        ..SyncCallbacks::default()
    };

    let svc = new_sync_service(inner, cache, opts, callbacks);
    let cancel = CancellationToken::new();

    let result = svc.sync_all(&cancel).await.expect("sync_all");
    assert_eq!(result.synced_projects, vec!["good".to_string()]);
    assert_eq!(result.failed_projects, vec!["bad".to_string()]);

    let callback_result = all_completed.lock().unwrap().clone().expect("callback");
    assert_eq!(callback_result.synced_projects, vec!["good".to_string()]);
    assert_eq!(callback_result.failed_projects, vec!["bad".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_service_uses_inner_client_not_caching_decorator() {
    let inner = Arc::new(SyncMockClient {
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("sync")) })
        })),
        ..SyncMockClient::new()
    });
    let shared = SharedSyncMockClient(Arc::clone(&inner));
    let cache = SyncMockCache::new();

    let svc = SyncService::new(
        shared.clone(),
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_project("demo", "en", &cancel)
        .await
        .expect("sync_project");
    assert_eq!(inner.get_project_call_count(), 1);

    let cache_for_decorator = SyncMockCache::new();
    {
        let mut state = cache_for_decorator.inner.lock().unwrap();
        state
            .projects
            .insert("demo:en".to_string(), test_project("cached"));
    }

    let decorated = CachingClient::new(
        shared,
        cache_for_decorator,
        CachingOptions {
            fallback_mode: FallbackMode::CacheFirst,
            default_project_id: "demo".into(),
        },
    )
    .expect("caching client");

    decorated
        .get_project("demo", "en", GetProjectOptions::new())
        .await
        .expect("cached read");
    assert_eq!(inner.get_project_call_count(), 1);

    decorated
        .get_project("demo", "fr", GetProjectOptions::new())
        .await
        .expect("uncached read");
    assert_eq!(inner.get_project_call_count(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_background_sync_no_op_when_auto_sync_disabled() {
    let inner = SyncMockClient::new();
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.auto_sync = false;

    let svc = Arc::new(new_sync_service(
        inner,
        cache,
        opts,
        SyncCallbacks::default(),
    ));

    svc.start_background_sync(CancellationToken::new());
    assert!(!svc.is_background_sync_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_sync_stops_on_cancellation_token_cancel() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move { Ok(demo_locales(&project, vec!["en".into()])) })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.projects = vec!["demo".into()];
    opts.auto_sync_interval = Some(Duration::from_millis(20));

    let svc = Arc::new(new_sync_service(
        inner,
        cache,
        opts,
        SyncCallbacks::default(),
    ));

    let parent = CancellationToken::new();
    svc.start_background_sync(parent.clone());
    assert!(svc.is_background_sync_running());

    tokio::time::sleep(Duration::from_millis(50)).await;
    parent.cancel();
    svc.stop_background_sync().await;

    assert!(!svc.is_background_sync_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_sync_stops_on_stop_background_sync() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move { Ok(demo_locales(&project, vec!["en".into()])) })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.projects = vec!["demo".into()];
    opts.auto_sync_interval = Some(Duration::from_millis(20));

    let svc = Arc::new(new_sync_service(
        inner,
        cache,
        opts,
        SyncCallbacks::default(),
    ));

    svc.start_background_sync(CancellationToken::new());
    assert!(svc.is_background_sync_running());

    tokio::time::sleep(Duration::from_millis(50)).await;
    svc.stop_background_sync().await;

    assert!(!svc.is_background_sync_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn background_sync_is_idempotent_start_stop() {
    let inner = SyncMockClient {
        get_project_locales_fn: Some(Box::new(|project, _| {
            let project = project.to_string();
            Box::pin(async move { Ok(demo_locales(&project, vec!["en".into()])) })
        })),
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };
    let cache = SyncMockCache::new();
    let mut opts = OfflineCacheOptions::default_offline_cache_options();
    opts.projects = vec!["demo".into()];
    opts.auto_sync_interval = Some(Duration::from_millis(20));

    let svc = Arc::new(new_sync_service(
        inner,
        cache,
        opts,
        SyncCallbacks::default(),
    ));

    let parent = CancellationToken::new();
    svc.start_background_sync(parent.clone());
    svc.start_background_sync(parent.clone());
    assert!(svc.is_background_sync_running());

    svc.stop_background_sync().await;
    svc.stop_background_sync().await;
    assert!(!svc.is_background_sync_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_project_file_provider_round_trip() {
    use translaas::cachefile::FileProvider;

    let dir = tempfile::tempdir().expect("tempdir");
    let file_cache = FileProvider::new(dir.path()).expect("file provider");

    let inner = SyncMockClient {
        get_project_fn: Some(Box::new(|_, _, _| {
            Box::pin(async { Ok(test_project("en")) })
        })),
        ..SyncMockClient::new()
    };

    let svc = SyncService::new(
        inner,
        file_cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_project("demo", "en", &cancel)
        .await
        .expect("sync to disk");

    let provider = FileProvider::new(dir.path()).expect("reopen provider");
    let cached = provider
        .get_project("demo", "en")
        .expect("read")
        .expect("project cached");
    assert!(cached.get_group("common").expect("group").is_some());
}
