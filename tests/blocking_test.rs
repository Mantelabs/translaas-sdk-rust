//! Wiremock coverage for `translaas::blocking` (issue #57).
//!
//! HTTP mocks run on a background multi-thread runtime. Product calls run on a
//! plain OS thread so `Handle::try_current()` is empty (consumer-shaped).

use translaas::blocking::ClientBuilder;
use translaas::client::{Error, GetEntryOptions};
use translaas::service::Error as ServiceError;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct LiveMock {
    runtime: tokio::runtime::Runtime,
    server: MockServer,
}

impl LiveMock {
    fn start() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("mock runtime");
        let server = runtime.block_on(MockServer::start());
        Self { runtime, server }
    }

    fn uri(&self) -> String {
        self.server.uri()
    }

    fn mount(&self, mock: Mock) {
        self.runtime.block_on(mock.mount(&self.server));
    }
}

fn on_sync_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::spawn(f).join().expect("sync thread")
}

fn blocking_builder(base_url: &str) -> ClientBuilder {
    ClientBuilder::new()
        .api_key("test-api-key")
        .base_url(base_url)
        .default_project_id("translaassdksamples")
        .default_language("en")
}

#[test]
fn blocking_builder_validation_errors() {
    let err = ClientBuilder::new().build().expect_err("api key required");
    assert!(err.message.contains("ApiKey"));
}

#[test]
fn blocking_get_entry_success() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .and(header("X-Api-Key", "test-api-key"))
            .and(query_param("group", "common"))
            .and(query_param("entry", "welcome.message"))
            .and(query_param("lang", "en"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Welcome back!")),
    );
    let uri = mock.uri();

    let text = on_sync_thread(move || {
        blocking_builder(&uri)
            .build()
            .expect("client")
            .get_entry("common", "welcome.message", "en", GetEntryOptions::new())
            .expect("get_entry")
    });
    assert_eq!(text, "Welcome back!");
}

#[test]
fn blocking_build_service_t() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .and(query_param("group", "common"))
            .and(query_param("entry", "welcome.message"))
            .and(query_param("lang", "en"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Welcome back!")),
    );
    let uri = mock.uri();

    let text = on_sync_thread(move || {
        blocking_builder(&uri)
            .build_service()
            .expect("service")
            .t("common", "welcome.message")
            .expect("t")
    });
    assert_eq!(text, "Welcome back!");
}

#[test]
fn blocking_t_lang_and_params() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .and(query_param("lang", "de"))
            .and(query_param("group", "common"))
            .and(query_param("entry", "welcome.message"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Willkommen!")),
    );
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .and(query_param("group", "messages"))
            .and(query_param("entry", "hello"))
            .and(query_param("name", "Ada"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello, Ada!")),
    );
    let uri = mock.uri();

    on_sync_thread(move || {
        let translaas = blocking_builder(&uri).build_service().expect("service");
        let lang = translaas
            .t_lang("common", "welcome.message", "de")
            .expect("t_lang");
        assert_eq!(lang, "Willkommen!");
        let greeting = translaas
            .t_params("messages", "hello", [("name", "Ada")])
            .expect("t_params");
        assert_eq!(greeting, "Hello, Ada!");
    });
}

#[test]
fn blocking_t_params_plural_hits_wiremock() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .and(query_param("group", "messages"))
            .and(query_param("entry", "item"))
            .and(query_param("N", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_string("5 items")),
    );
    let uri = mock.uri();

    let text = on_sync_thread(move || {
        blocking_builder(&uri)
            .build_service()
            .expect("service")
            .t_params("messages", "item", 5)
            .expect("plural")
    });
    assert_eq!(text, "5 items");
}

#[test]
fn blocking_api_error_is_not_transport() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found")),
    );
    let uri = mock.uri();

    on_sync_thread(move || {
        let err = blocking_builder(&uri)
            .build_service()
            .expect("service")
            .t_lang("common", "welcome.message", "en")
            .expect_err("api");
        match err {
            ServiceError::Client(client_err) => {
                let api = client_err.as_api().expect("Error::Api");
                assert_eq!(api.status_code, 404);
                assert!(!client_err.is_transport());
                assert!(!client_err.is_blocking_in_async_context());
            }
            other => panic!("expected Client, got {other:?}"),
        }
    });
}

#[test]
fn blocking_dns_failure_is_transport() {
    let uri = "http://translaas-does-not-exist.invalid".to_string();

    on_sync_thread(move || {
        let err = blocking_builder(&uri)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("client")
            .get_entry("common", "welcome.message", "en", GetEntryOptions::new())
            .expect_err("dns");
        assert!(err.is_transport(), "got {err:?}");
        assert!(err.as_api().is_none());
        assert!(!err.is_blocking_in_async_context());
    });
}

#[tokio::test]
async fn blocking_rejects_foreign_runtime() {
    let uri = "https://example.invalid".to_string();
    let translaas = std::thread::spawn(move || blocking_builder(&uri).build_service())
        .join()
        .expect("spawn")
        .expect("service");

    let err = translaas
        .t("common", "welcome.message")
        .expect_err("foreign runtime");
    match err {
        ServiceError::Client(client_err) => {
            assert!(client_err.is_blocking_in_async_context());
            assert!(!client_err.is_transport());
            assert!(client_err.as_api().is_none());
        }
        other => panic!("expected Client, got {other:?}"),
    }

    std::thread::spawn(move || drop(translaas))
        .join()
        .expect("drop service off async thread");
}

#[test]
fn blocking_error_not_transport_or_api() {
    let err = Error::BlockingInAsyncContext;
    assert!(err.is_blocking_in_async_context());
    assert!(!err.is_transport());
    assert!(err.as_api().is_none());
}

#[test]
fn blocking_nested_facade_uses_single_block_on() {
    let mock = LiveMock::start();
    mock.mount(
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok")),
    );
    let uri = mock.uri();
    let text = on_sync_thread(move || {
        blocking_builder(&uri)
            .build_service()
            .expect("service")
            .t("common", "welcome.message")
            .expect("t")
    });
    assert_eq!(text, "ok");
}
