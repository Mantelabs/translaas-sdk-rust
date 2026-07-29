//! Integration tests for `SyncService::sync_from_offline_zip`.

#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio_util::sync::CancellationToken;
use translaas::cachefile::{
    FileProvider, HybridOptions, HybridProvider, OfflineCacheOptions, Provider, SaveOptions,
    SyncCallbacks, SyncCompletedEvent, SyncFailedEvent, SyncService,
};
use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    ConfigurationError, OfflineCacheDownloadResult, OfflineCacheError, ProjectLocales,
    ReportMissingKeyItem, TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};

#[path = "support/offline_zip.rs"]
mod offline_zip;

use offline_zip::build_test_offline_zip;

struct SyncMockClient {
    state: Mutex<SyncMockClientState>,
    get_offline_cache_fn: Option<
        Box<
            dyn Fn(
                    &str,
                    GetOfflineCacheOptions<'_>,
                ) -> std::pin::Pin<
                    Box<
                        dyn std::future::Future<Output = Result<OfflineCacheDownloadResult, Error>>
                            + Send,
                    >,
                > + Send
                + Sync,
        >,
    >,
}

#[derive(Default)]
struct SyncMockClientState {
    get_offline_cache_calls: u32,
}

impl SyncMockClient {
    fn new() -> Self {
        Self {
            state: Mutex::new(SyncMockClientState::default()),
            get_offline_cache_fn: None,
        }
    }

    fn get_offline_cache_call_count(&self) -> u32 {
        self.state.lock().unwrap().get_offline_cache_calls
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
        _: &str,
        _: &str,
        _: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        Err(unexpected("get_project"))
    }

    async fn get_project_locales(
        &self,
        _: &str,
        _: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        Err(unexpected("get_project_locales"))
    }

    async fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        self.state.lock().unwrap().get_offline_cache_calls += 1;
        if let Some(ref handler) = self.get_offline_cache_fn {
            return handler(project, opts).await;
        }
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

    fn get_locales(&self, _: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        Ok(None)
    }

    fn save_locales(
        &self,
        _: &str,
        _: &ProjectLocales,
        _: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        self.inner.lock().unwrap().save_locales_calls += 1;
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

fn new_test_provider() -> FileProvider {
    let dir = tempfile::tempdir().expect("tempdir");
    FileProvider::new(dir.keep()).expect("provider")
}

#[tokio::test]
async fn sync_from_offline_zip_imports_bundle() {
    let zip_bytes = build_test_offline_zip();
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(move |project, _| {
        let zip_bytes = zip_bytes.clone();
        let project = project.to_string();
        Box::pin(async move {
            if project != "demo-project" {
                panic!("unexpected project {project}");
            }
            Ok(OfflineCacheDownloadResult {
                content: Some(zip_bytes),
                not_modified: false,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let cache = new_test_provider();
    let svc = SyncService::new(
        client,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_from_offline_zip("demo-project", &cancel)
        .await
        .expect("sync");

    let got = svc
        .cache()
        .get_project("demo-project", "en")
        .expect("get")
        .expect("project");
    assert!(got.groups.contains_key("common"));
}

#[tokio::test]
async fn sync_from_offline_zip_not_modified_no_op() {
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(|_, _| {
        Box::pin(async {
            Ok(OfflineCacheDownloadResult {
                content: None,
                not_modified: true,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let cache = new_test_provider();
    let svc = SyncService::new(
        client,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );
    let cancel = CancellationToken::new();

    svc.sync_from_offline_zip("demo-project", &cancel)
        .await
        .expect("sync");

    assert_eq!(svc.client().get_offline_cache_call_count(), 1);
    assert!(svc
        .cache()
        .get_project("demo-project", "en")
        .expect("get")
        .is_none());
}

#[tokio::test]
async fn sync_from_offline_zip_empty_content_no_op() {
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(|_, _| {
        Box::pin(async {
            Ok(OfflineCacheDownloadResult {
                content: Some(Vec::new()),
                not_modified: false,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let cache = new_test_provider();
    let svc = SyncService::new(
        client,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );

    svc.sync_from_offline_zip("demo-project", &CancellationToken::new())
        .await
        .expect("sync");

    assert!(svc
        .cache()
        .get_project("demo-project", "en")
        .expect("get")
        .is_none());
}

#[tokio::test]
async fn sync_from_offline_zip_raises_failed_callback() {
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(|_, _| {
        Box::pin(async {
            Err(Error::Configuration(ConfigurationError {
                message: "download failed".to_string(),
            }))
        })
    }));

    let cache = new_test_provider();
    let failed = Arc::new(Mutex::new(None::<SyncFailedEvent>));
    let failed_cb = Arc::clone(&failed);
    let svc = SyncService::new(
        client,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks {
            on_sync_failed: Some(Arc::new(move |event| {
                *failed_cb.lock().unwrap() = Some(event);
            })),
            ..SyncCallbacks::default()
        },
    );

    let err = svc
        .sync_from_offline_zip("demo-project", &CancellationToken::new())
        .await
        .expect_err("download error");
    assert!(err.to_string().contains("download failed"));

    let event = failed.lock().unwrap().take().expect("callback");
    assert_eq!(event.project, "demo-project");
}

#[tokio::test]
async fn sync_from_offline_zip_generic_provider_fallback() {
    let zip_bytes = build_test_offline_zip();
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(move |_, _| {
        let zip_bytes = zip_bytes.clone();
        Box::pin(async move {
            Ok(OfflineCacheDownloadResult {
                content: Some(zip_bytes),
                not_modified: false,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let cache = SyncMockCache::new();
    let svc = SyncService::new(
        client,
        cache,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );

    svc.sync_from_offline_zip("demo-project", &CancellationToken::new())
        .await
        .expect("sync");

    assert_eq!(svc.cache().save_project_call_count(), 2);
    assert_eq!(svc.cache().save_locales_call_count(), 1);
}

#[tokio::test]
async fn sync_from_offline_zip_completed_callback() {
    let zip_bytes = build_test_offline_zip();
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(move |_, _| {
        let zip_bytes = zip_bytes.clone();
        Box::pin(async move {
            Ok(OfflineCacheDownloadResult {
                content: Some(zip_bytes),
                not_modified: false,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let completed = Arc::new(Mutex::new(None::<SyncCompletedEvent>));
    let completed_cb = Arc::clone(&completed);
    let svc = SyncService::new(
        client,
        new_test_provider(),
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks {
            on_sync_completed: Some(Arc::new(move |event| {
                *completed_cb.lock().unwrap() = Some(event);
            })),
            ..SyncCallbacks::default()
        },
    );

    svc.sync_from_offline_zip("demo-project", &CancellationToken::new())
        .await
        .expect("sync");

    let event = completed.lock().unwrap().take().expect("completed");
    assert_eq!(event.project, "demo-project");
    assert!(event.language.is_empty());
}

#[tokio::test]
async fn sync_from_offline_zip_updates_hybrid_l1() {
    let zip_bytes = build_test_offline_zip();
    let mut client = SyncMockClient::new();
    client.get_offline_cache_fn = Some(Box::new(move |_, _| {
        let zip_bytes = zip_bytes.clone();
        Box::pin(async move {
            Ok(OfflineCacheDownloadResult {
                content: Some(zip_bytes),
                not_modified: false,
                etag: None,
                suggested_file_name: None,
            })
        })
    }));

    let dir = tempfile::tempdir().expect("tempdir");
    let file = FileProvider::new(dir.path()).expect("file");
    let hybrid = HybridProvider::new(file, HybridOptions::default());

    let mut stale = HashMap::new();
    stale.insert("common".to_string(), serde_json::json!({"hello": "Stale"}));
    hybrid
        .save_project(
            "demo-project",
            "en",
            &TranslationProject {
                groups: stale,
                ..Default::default()
            },
            SaveOptions::new(),
        )
        .expect("seed stale");

    let svc = SyncService::new(
        client,
        hybrid,
        OfflineCacheOptions::default_offline_cache_options(),
        SyncCallbacks::default(),
    );

    svc.sync_from_offline_zip("demo-project", &CancellationToken::new())
        .await
        .expect("sync");

    let got = svc
        .cache()
        .get_project("demo-project", "en")
        .expect("get")
        .expect("project");
    let group = got.get_group("common").expect("group").expect("some");
    let hello = group
        .entries
        .get("hello")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(hello, "Hello");
}
