//! In-memory cache integration tests (issue #7).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use translaas::cache::{entry_key, CacheMode, MemoryOptions, MemoryProvider, Provider, Ttl};
use translaas::client::{
    Client, GetEntryOptions, GetGroupOptions, GetProjectLocalesOptions, GetProjectOptions,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn new_cached_client(
    server: &MockServer,
    mode: CacheMode,
    provider: Arc<MemoryProvider>,
) -> Client {
    Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .cache_mode(mode)
        .cache_provider(provider)
        .build()
        .expect("client")
}

#[tokio::test]
async fn default_memory_provider_when_cache_enabled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("cached-default"))
        .expect(1)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .cache_mode(CacheMode::Entry)
        .build()
        .expect("client");

    for _ in 0..2 {
        let got = client
            .get_entry("ui", "greeting", "en", GetEntryOptions::new())
            .await
            .unwrap();
        assert_eq!(got, "cached-default");
    }
}

#[tokio::test]
async fn get_entry_cache_hit_miss() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Entry, Arc::clone(&provider)).await;

    let got = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(got, "Hello, World!");

    let got = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(got, "Hello, World!");
}

#[tokio::test]
async fn get_entry_not_cached_in_group_mode() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("value"))
        .expect(2)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Group, provider).await;

    for _ in 0..2 {
        let got = client
            .get_entry("ui", "greeting", "en", GetEntryOptions::new())
            .await
            .unwrap();
        assert_eq!(got, "value");
    }
}

#[tokio::test]
async fn get_group_cache_hit_miss() {
    let server = MockServer::start().await;
    let body = r#"{"Entries":{"welcome":"Welcome"}}"#;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Group, provider).await;

    let got = client
        .get_group("p", "ui", "en", GetGroupOptions::new())
        .await
        .unwrap();
    assert_eq!(got.get_value("welcome"), Some("Welcome"));

    let got = client
        .get_group("p", "ui", "en", GetGroupOptions::new())
        .await
        .unwrap();
    assert_eq!(got.get_value("welcome"), Some("Welcome"));
}

#[tokio::test]
async fn get_project_cache_hit_miss() {
    let server = MockServer::start().await;
    let body = r#"{"Groups":{"ui":{"title":"Checkout"}}}"#;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/project"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Project, provider).await;

    client
        .get_project("p", "en", GetProjectOptions::new())
        .await
        .unwrap();
    client
        .get_project("p", "en", GetProjectOptions::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_project_locales_cached_in_entry_mode() {
    let server = MockServer::start().await;
    let body = r#"{"locales":["en","fr"]}"#;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/locales"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Entry, provider).await;

    let got = client
        .get_project_locales("p", GetProjectLocalesOptions::new())
        .await
        .unwrap();
    assert_eq!(got.locales.len(), 2);

    client
        .get_project_locales("p", GetProjectLocalesOptions::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_entry_304_returns_cached_value() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(304))
        .expect(0)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    provider
        .set(
            &entry_key(
                "ui",
                "greeting",
                "en",
                None,
                &Default::default(),
                "",
                "",
                "",
            ),
            "Cached greeting".to_string(),
            Ttl::none(),
        )
        .expect("seed cache");

    let client = new_cached_client(&server, CacheMode::Entry, provider).await;
    let got = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(got, "Cached greeting");
}

#[tokio::test]
async fn get_entry_304_without_cache_returns_empty() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Entry, provider).await;
    let got = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(got, "");
}

#[tokio::test]
async fn get_group_304_does_not_poison_cache() {
    let server = MockServer::start().await;
    let body = r#"{"Entries":{"title":"Original"}}"#;
    let request_count = Arc::new(AtomicU32::new(0));

    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with({
            let request_count = Arc::clone(&request_count);
            move |_req: &wiremock::Request| {
                let count = request_count.fetch_add(1, Ordering::SeqCst) + 1;
                if count == 1 {
                    ResponseTemplate::new(200).set_body_string(body)
                } else {
                    ResponseTemplate::new(304)
                }
            }
        })
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Group, provider).await;

    let got = client
        .get_group("p", "ui", "en", GetGroupOptions::new())
        .await
        .unwrap();
    assert_eq!(got.get_value("title"), Some("Original"));

    for _ in 0..2 {
        let got = client
            .get_group("p", "ui", "en", GetGroupOptions::new())
            .await
            .unwrap();
        assert_eq!(got.get_value("title"), Some("Original"));
    }
}

#[tokio::test]
async fn get_entry_cache_absolute_expiry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("value"))
        .expect(2)
        .mount(&server)
        .await;

    let base = std::time::Instant::now();
    let clock = Arc::new(std::sync::Mutex::new(base));
    let clock_for_provider = Arc::clone(&clock);
    let provider = Arc::new(MemoryProvider::with_options(
        MemoryOptions::default()
            .with_clock(Arc::new(move || *clock_for_provider.lock().expect("clock"))),
    ));

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .cache_mode(CacheMode::Entry)
        .cache_ttl(Ttl::absolute(Duration::from_millis(20)))
        .cache_provider(provider)
        .build()
        .expect("client");

    client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();

    {
        let mut now = clock.lock().expect("clock");
        *now += Duration::from_millis(30);
    }

    client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn validate_api_key_not_cached() {
    let server = MockServer::start().await;
    let fixture = std::fs::read_to_string(format!(
        "{}/testdata/validate_api_key_tenant.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("read fixture");
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture))
        .expect(2)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Project, provider).await;

    for _ in 0..2 {
        client.validate_api_key().await.unwrap();
    }
}

#[tokio::test]
async fn get_entry_cached_value_isolated_from_mutation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(200).set_body_string("original"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Entry, provider).await;

    let mut first = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    first.push_str("-mutated");

    let second = client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(second, "original");
}

#[tokio::test]
async fn get_group_cached_value_isolated_from_mutation() {
    let server = MockServer::start().await;
    let body = r#"{"Entries":{"title":"Original"}}"#;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
        .expect(1)
        .mount(&server)
        .await;

    let provider = Arc::new(MemoryProvider::new());
    let client = new_cached_client(&server, CacheMode::Group, provider).await;

    let mut first = client
        .get_group("p", "ui", "en", GetGroupOptions::new())
        .await
        .unwrap();
    first.entries.clear();

    let second = client
        .get_group("p", "ui", "en", GetGroupOptions::new())
        .await
        .unwrap();
    assert_eq!(second.get_value("title"), Some("Original"));
}
