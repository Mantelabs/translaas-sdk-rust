//! Cache-only / cache-first `get_entry` CLDR plural selection.

use std::collections::HashMap;
use std::sync::Mutex;

use serde_json::Value;
use translaas::cachefile::{CachingClient, CachingOptions, FallbackMode, Provider, SaveOptions};
use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    OfflineCacheDownloadResult, OfflineCacheError, ProjectLocales, ReportMissingKeyItem,
    TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};

const TEST_PROJECT_ID: &str = "demo-project";

const GOLDEN_GET_ENTRY_ROWS: &[(&str, f64, &str)] = &[
    ("ar", 0.0, "FORM:zero"),
    ("ar", 2.0, "FORM:two"),
    ("pl", 2.0, "FORM:few"),
    ("fr", 0.0, "FORM:one"),
    ("en", 0.0, "FORM:other"),
    ("en", 1.0, "FORM:one"),
    ("pt", 0.0, "FORM:one"),
    ("pt-PT", 0.0, "FORM:other"),
];

struct CountingInner {
    get_entry_calls: Mutex<u32>,
}

impl CountingInner {
    fn new() -> Self {
        Self {
            get_entry_calls: Mutex::new(0),
        }
    }

    fn get_entry_call_count(&self) -> u32 {
        *self.get_entry_calls.lock().unwrap()
    }
}

impl TranslaasClient for CountingInner {
    async fn get_entry(
        &self,
        _group: &str,
        _entry: &str,
        _lang: &str,
        _opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        *self.get_entry_calls.lock().unwrap() += 1;
        Err(Error::Configuration(
            translaas::models::ConfigurationError {
                message: "unexpected get_entry".to_string(),
            },
        ))
    }

    async fn get_group(
        &self,
        _project: &str,
        _group: &str,
        _lang: &str,
        _opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
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
        unexpected("get_project")
    }

    async fn get_project_locales(
        &self,
        _project: &str,
        _opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        unexpected("get_project_locales")
    }

    async fn get_offline_cache(
        &self,
        _project: &str,
        _opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        unexpected("get_offline_cache")
    }

    async fn report_missing_keys(&self, _keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        unexpected("report_missing_keys")
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        unexpected("validate_api_key")
    }
}

fn unexpected<T>(op: &str) -> Result<T, Error> {
    Err(Error::Configuration(
        translaas::models::ConfigurationError {
            message: format!("unexpected {op}"),
        },
    ))
}

struct MapCache {
    groups: Mutex<HashMap<String, TranslationGroup>>,
}

impl MapCache {
    fn new() -> Self {
        Self {
            groups: Mutex::new(HashMap::new()),
        }
    }

    fn insert_group(&self, group: &str, lang: &str, data: TranslationGroup) {
        self.groups
            .lock()
            .unwrap()
            .insert(group_key(TEST_PROJECT_ID, group, lang), data);
    }
}

fn group_key(project: &str, group: &str, lang: &str) -> String {
    format!("{project}:{group}:{lang}")
}

impl Provider for MapCache {
    fn get_project(
        &self,
        _project: &str,
        _lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        Ok(None)
    }

    fn save_project(
        &self,
        _project: &str,
        _lang: &str,
        _data: &TranslationProject,
        _options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        Ok(())
    }

    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        Ok(self
            .groups
            .lock()
            .unwrap()
            .get(&group_key(project, group, lang))
            .cloned())
    }

    fn get_locales(&self, _project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        Ok(None)
    }

    fn save_locales(
        &self,
        _project: &str,
        _data: &ProjectLocales,
        _options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
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

fn new_client(
    inner: CountingInner,
    cache: MapCache,
    mode: FallbackMode,
) -> CachingClient<CountingInner, MapCache> {
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

fn plural_group(entry: &str, forms: &[(&str, &str)]) -> TranslationGroup {
    let mut map = serde_json::Map::new();
    for (category, text) in forms {
        map.insert((*category).to_string(), Value::String((*text).to_string()));
    }
    let mut entries = HashMap::new();
    entries.insert(entry.to_string(), Value::Object(map));
    TranslationGroup {
        entries,
        ..Default::default()
    }
}

fn all_plural_forms(entry: &str) -> TranslationGroup {
    plural_group(
        entry,
        &[
            ("zero", "FORM:zero"),
            ("one", "FORM:one"),
            ("two", "FORM:two"),
            ("few", "FORM:few"),
            ("many", "FORM:many"),
            ("other", "FORM:other"),
        ],
    )
}

fn string_group(entry: &str, value: &str) -> TranslationGroup {
    let mut entries = HashMap::new();
    entries.insert(entry.to_string(), Value::String(value.to_string()));
    TranslationGroup {
        entries,
        ..Default::default()
    }
}

#[tokio::test]
async fn get_entry_cache_only_cldr_golden() {
    for (lang, n, expected) in GOLDEN_GET_ENTRY_ROWS {
        let inner = CountingInner::new();
        let cache = MapCache::new();
        cache.insert_group("messages", lang, all_plural_forms("items"));
        let client = new_client(inner, cache, FallbackMode::CacheOnly);
        let got = client
            .get_entry("messages", "items", lang, GetEntryOptions::new().number(*n))
            .await
            .unwrap_or_else(|err| panic!("get_entry {lang} n={n}: {err}"));
        assert_eq!(got, *expected, "golden {lang} n={n}");
        assert_eq!(client.inner().get_entry_call_count(), 0);
    }
}

#[tokio::test]
async fn get_entry_cache_only_falls_back_to_other_when_category_missing() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group(
        "messages",
        "ar",
        plural_group("items", &[("other", "FORM:other-only")]),
    );
    let client = new_client(inner, cache, FallbackMode::CacheOnly);
    let got = client
        .get_entry(
            "messages",
            "items",
            "ar",
            GetEntryOptions::new().number(2.0),
        )
        .await
        .expect("get_entry");
    assert_eq!(got, "FORM:other-only");
    assert_eq!(client.inner().get_entry_call_count(), 0);
}

#[tokio::test]
async fn get_entry_cache_only_null_number_uses_other_form() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group("messages", "en", all_plural_forms("items"));
    let client = new_client(inner, cache, FallbackMode::CacheOnly);
    let got = client
        .get_entry("messages", "items", "en", GetEntryOptions::new())
        .await
        .expect("get_entry");
    assert_eq!(got, "FORM:other");
}

#[tokio::test]
async fn get_entry_cache_only_still_substitutes_n() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group(
        "messages",
        "en",
        plural_group("items", &[("other", "Count {N}")]),
    );
    let client = new_client(inner, cache, FallbackMode::CacheOnly);
    let got = client
        .get_entry(
            "messages",
            "items",
            "en",
            GetEntryOptions::new().number(0.0),
        )
        .await
        .expect("get_entry");
    assert_eq!(got, "Count 0");
}

#[tokio::test]
async fn get_entry_cache_only_non_plural_unchanged() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group("common", "en", string_group("hello", "Hello World"));
    let client = new_client(inner, cache, FallbackMode::CacheOnly);
    let got = client
        .get_entry("common", "hello", "en", GetEntryOptions::new().number(5.0))
        .await
        .expect("get_entry");
    assert_eq!(got, "Hello World");
}

#[tokio::test]
async fn get_entry_cache_first_returns_cldr_plural_on_hit() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group("messages", "pl", all_plural_forms("items"));
    let client = new_client(inner, cache, FallbackMode::CacheFirst);
    let got = client
        .get_entry(
            "messages",
            "items",
            "pl",
            GetEntryOptions::new().number(2.0),
        )
        .await
        .expect("get_entry");
    assert_eq!(got, "FORM:few");
    assert_eq!(client.inner().get_entry_call_count(), 0);
}

#[tokio::test]
async fn get_entry_cache_first_uses_pt_pt_rules() {
    let inner = CountingInner::new();
    let cache = MapCache::new();
    cache.insert_group("messages", "pt-PT", all_plural_forms("items"));
    let client = new_client(inner, cache, FallbackMode::CacheFirst);
    let got = client
        .get_entry(
            "messages",
            "items",
            "pt-PT",
            GetEntryOptions::new().number(0.0),
        )
        .await
        .expect("get_entry");
    assert_eq!(got, "FORM:other");
    assert_eq!(client.inner().get_entry_call_count(), 0);
}
