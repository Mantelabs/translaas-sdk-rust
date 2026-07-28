//! Integration tests for `cachefile::CachingClient` fallback modes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use translaas::cachefile::{
    CachingClient, CachingOptions, FallbackMode, FileProvider, OfflineStubClient, Provider,
    SaveOptions,
};
use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    ApiError, OfflineCacheDownloadResult, OfflineCacheError, ProjectLocales, ReportMissingKeyItem,
    TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};

const TEST_PROJECT_ID: &str = "demo-project";

struct MockInnerClient {
    state: Mutex<MockInnerState>,
    get_entry_fn: Option<
        Box<
            dyn Fn(
                    &str,
                    &str,
                    &str,
                    GetEntryOptions<'_>,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<String, Error>> + Send>,
                > + Send
                + Sync,
        >,
    >,
    get_group_fn: Option<
        Box<
            dyn Fn(
                    &str,
                    &str,
                    &str,
                    GetGroupOptions<'_>,
                ) -> std::pin::Pin<
                    Box<dyn std::future::Future<Output = Result<TranslationGroup, Error>> + Send>,
                > + Send
                + Sync,
        >,
    >,
    validate_api_key_fn: Option<
        Box<
            dyn Fn() -> std::pin::Pin<
                    Box<
                        dyn std::future::Future<Output = Result<ValidateApiKeyResponse, Error>>
                            + Send,
                    >,
                > + Send
                + Sync,
        >,
    >,
}

#[derive(Default)]
struct MockInnerState {
    get_entry_calls: u32,
    get_group_calls: u32,
}

impl MockInnerClient {
    fn new() -> Self {
        Self {
            state: Mutex::new(MockInnerState::default()),
            get_entry_fn: None,
            get_group_fn: None,
            validate_api_key_fn: None,
        }
    }

    fn get_entry_call_count(&self) -> u32 {
        self.state.lock().unwrap().get_entry_calls
    }

    fn get_group_call_count(&self) -> u32 {
        self.state.lock().unwrap().get_group_calls
    }
}

impl TranslaasClient for MockInnerClient {
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        self.state.lock().unwrap().get_entry_calls += 1;
        if let Some(ref handler) = self.get_entry_fn {
            return handler(group, entry, lang, opts).await;
        }
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_entry".to_string(),
            },
        ))
    }

    async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        self.state.lock().unwrap().get_group_calls += 1;
        if let Some(ref handler) = self.get_group_fn {
            return handler(project, group, lang, opts).await;
        }
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_group".to_string(),
            },
        ))
    }

    async fn get_project(
        &self,
        _project: &str,
        _lang: &str,
        _opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_project".to_string(),
            },
        ))
    }

    async fn get_project_locales(
        &self,
        _project: &str,
        _opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_project_locales".to_string(),
            },
        ))
    }

    async fn get_offline_cache(
        &self,
        _project: &str,
        _opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_offline_cache".to_string(),
            },
        ))
    }

    async fn report_missing_keys(&self, _keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected report_missing_keys".to_string(),
            },
        ))
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        if let Some(ref handler) = self.validate_api_key_fn {
            return handler().await;
        }
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected validate_api_key".to_string(),
            },
        ))
    }
}

struct MockCacheProvider {
    inner: Mutex<MockCacheState>,
}

#[derive(Default)]
struct MockCacheState {
    save_project_calls: u32,
    groups: HashMap<String, TranslationGroup>,
    projects: HashMap<String, TranslationProject>,
    locales: HashMap<String, ProjectLocales>,
    get_group_error: bool,
}

impl MockCacheProvider {
    fn new() -> Self {
        Self {
            inner: Mutex::new(MockCacheState::default()),
        }
    }

    fn save_project_count(&self) -> u32 {
        self.inner.lock().unwrap().save_project_calls
    }
}

fn group_key(project: &str, group: &str, lang: &str) -> String {
    format!("{project}:{group}:{lang}")
}

impl Provider for MockCacheProvider {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        let state = self.inner.lock().unwrap();
        Ok(state.projects.get(&format!("{project}:{lang}")).cloned())
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        _options: SaveOptions,
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
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        let mut state = self.inner.lock().unwrap();
        if state.get_group_error {
            return Err(OfflineCacheError::new(
                "corrupt cache",
                None,
                Some(project.to_string()),
                Some(lang.to_string()),
                None,
            ));
        }
        Ok(state.groups.get(&group_key(project, group, lang)).cloned())
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        let state = self.inner.lock().unwrap();
        Ok(state.locales.get(project).cloned())
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        _options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        let mut state = self.inner.lock().unwrap();
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

    fn is_cached(&self, _project: &str, _lang: &str) -> Result<bool, OfflineCacheError> {
        Ok(false)
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        Ok(())
    }
}

fn test_group(entry: &str, value: &str) -> TranslationGroup {
    let mut entries = HashMap::new();
    entries.insert(entry.to_string(), Value::String(value.to_string()));
    TranslationGroup {
        entries,
        ..Default::default()
    }
}

fn new_caching_client(
    inner: MockInnerClient,
    cache: MockCacheProvider,
    mode: FallbackMode,
) -> CachingClient<MockInnerClient, MockCacheProvider> {
    CachingClient::new(
        inner,
        cache,
        CachingOptions {
            fallback_mode: mode,
            default_project_id: TEST_PROJECT_ID.to_string(),
        },
    )
    .expect("valid caching client")
}

#[tokio::test]
async fn new_caching_client_validation() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();

    assert!(CachingClient::new(
        inner,
        cache,
        CachingOptions {
            fallback_mode: FallbackMode::CacheFirst,
            default_project_id: String::new(),
        }
    )
    .is_err());
}

#[tokio::test]
async fn get_entry_cache_first_hit() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "common", "en"),
            test_group("hello", "Hello World"),
        );
    }

    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "Hello World");
    assert_eq!(client.inner().get_entry_call_count(), 0);
}

#[tokio::test]
async fn get_entry_cache_first_miss_calls_api() {
    let mut inner = MockInnerClient::new();
    inner.get_entry_fn = Some(Box::new(|_group, _entry, _lang, _opts| {
        Box::pin(async { Ok("Hello from API".to_string()) })
    }));
    inner.get_group_fn = Some(Box::new(|_project, _group, _entry, _opts| {
        Box::pin(async { Ok(test_group("hello", "Hello from API")) })
    }));

    let cache = MockCacheProvider::new();
    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "Hello from API");
    assert_eq!(client.inner().get_entry_call_count(), 1);
    assert_eq!(client.cache().save_project_count(), 1);
}

#[tokio::test]
async fn get_entry_cache_first_api_failure_returns_miss() {
    let mut inner = MockInnerClient::new();
    inner.get_entry_fn = Some(Box::new(|_group, _entry, _lang, _opts| {
        Box::pin(async {
            Err(Error::Api(ApiError {
                status_code: 502,
                code: None,
                message: Some("network".to_string()),
                response_content: None,
            }))
        })
    }));

    let cache = MockCacheProvider::new();
    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let err = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect_err("expected miss");
    assert!(err.is_offline_cache_miss());
}

#[tokio::test]
async fn get_entry_api_first_falls_back_to_cache() {
    let mut inner = MockInnerClient::new();
    inner.get_entry_fn = Some(Box::new(|_group, _entry, _lang, _opts| {
        Box::pin(async {
            Err(Error::Api(ApiError {
                status_code: 502,
                code: None,
                message: Some("network".to_string()),
                response_content: None,
            }))
        })
    }));

    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "common", "en"),
            test_group("hello", "Hello from Cache"),
        );
    }

    let client = new_caching_client(inner, cache, FallbackMode::ApiFirst);
    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "Hello from Cache");
}

#[tokio::test]
async fn get_entry_cache_only_no_api() {
    let mut inner = MockInnerClient::new();
    inner.get_entry_fn = Some(Box::new(|_group, _entry, _lang, _opts| {
        Box::pin(async {
            panic!("API should not be called in CacheOnly mode");
        })
    }));

    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "common", "en"),
            test_group("hello", "Cached"),
        );
    }

    let client = new_caching_client(inner, cache, FallbackMode::CacheOnly);
    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "Cached");
}

#[tokio::test]
async fn get_entry_cache_only_miss() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    let client = new_caching_client(inner, cache, FallbackMode::CacheOnly);
    let err = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect_err("expected miss");
    assert!(err.is_offline_cache_miss());
}

#[tokio::test]
async fn get_entry_cache_only_parameter_substitution() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "messages", "en"),
            test_group("greeting", "Hello {userName}, you have {N} items"),
        );
    }

    let client = new_caching_client(inner, cache, FallbackMode::CacheOnly);
    let mut params = HashMap::new();
    params.insert("userName".to_string(), "John".to_string());
    let got = client
        .get_entry(
            "messages",
            "greeting",
            "en",
            GetEntryOptions::new().number(5.0).parameters(params),
        )
        .await
        .expect("get_entry");
    assert_eq!(got, "Hello John, you have 5 items");
}

#[tokio::test]
async fn get_group_cache_first_hit() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "common", "en"),
            test_group("hello", "Hi"),
        );
    }

    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let group = client
        .get_group(TEST_PROJECT_ID, "common", "en", GetGroupOptions::new())
        .await
        .expect("get_group");
    assert_eq!(group.get_value("hello"), Some("Hi"));
    assert_eq!(client.inner().get_group_call_count(), 0);
}

#[tokio::test]
async fn get_project_locales_cache_only_uses_offline_cache_error() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    let client = new_caching_client(inner, cache, FallbackMode::CacheOnly);
    let err = client
        .get_project_locales(TEST_PROJECT_ID, GetProjectLocalesOptions::new())
        .await
        .expect_err("expected offline cache error");
    assert!(matches!(err, Error::OfflineCache(_)));
}

#[tokio::test]
async fn passthrough_validate_api_key() {
    let mut inner = MockInnerClient::new();
    inner.validate_api_key_fn = Some(Box::new(|| {
        Box::pin(async {
            Ok(ValidateApiKeyResponse {
                is_valid: true,
                tenant_id: None,
                project_id: None,
                project_ids: None,
                default_project_id: None,
                integration_name: None,
                authenticated_at: None,
            })
        })
    }));

    let cache = MockCacheProvider::new();
    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let resp = client.validate_api_key().await.expect("validate_api_key");
    assert!(resp.is_valid);
}

#[tokio::test]
async fn caching_client_integration_with_file_provider() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file_provider = FileProvider::new(dir.path()).expect("file provider");

    let mut project = TranslationProject::default();
    project.groups.insert(
        "common".to_string(),
        serde_json::json!({"hello": "Offline Hello"}),
    );
    file_provider
        .save_project(TEST_PROJECT_ID, "en", &project, SaveOptions::new())
        .expect("save_project");

    let inner = OfflineStubClient::new();
    let client = CachingClient::new(
        inner,
        file_provider,
        CachingOptions {
            fallback_mode: FallbackMode::CacheOnly,
            default_project_id: TEST_PROJECT_ID.to_string(),
        },
    )
    .expect("caching client");

    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "Offline Hello");
}

#[tokio::test]
async fn caching_client_concurrent_get_entry() {
    let inner = MockInnerClient::new();
    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.groups.insert(
            group_key(TEST_PROJECT_ID, "common", "en"),
            test_group("hello", "Hi"),
        );
    }

    let client = Arc::new(new_caching_client(inner, cache, FallbackMode::CacheFirst));
    let mut handles = Vec::new();
    for _ in 0..16 {
        let client = Arc::clone(&client);
        handles.push(tokio::spawn(async move {
            let _ = client
                .get_entry("common", "hello", "en", GetEntryOptions::new())
                .await;
        }));
    }
    for handle in handles {
        handle.await.expect("task");
    }
}

#[tokio::test]
async fn disk_error_propagates_without_api_fallback() {
    let mut inner = MockInnerClient::new();
    inner.get_entry_fn = Some(Box::new(|_group, _entry, _lang, _opts| {
        Box::pin(async {
            panic!("inner should not be called when disk errors");
        })
    }));

    let cache = MockCacheProvider::new();
    {
        let mut state = cache.inner.lock().unwrap();
        state.get_group_error = true;
    }

    let client = new_caching_client(inner, cache, FallbackMode::CacheFirst);
    let err = client
        .get_entry("common", "hello", "en", GetEntryOptions::new())
        .await
        .expect_err("expected disk error");
    assert!(matches!(err, Error::OfflineCache(_)));
}
