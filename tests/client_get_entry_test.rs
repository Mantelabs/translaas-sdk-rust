//! Mock HTTP coverage for `Client::get_entry` (issue #4).

use std::collections::HashMap;
use std::time::Duration;

use serde_json::json;
use translaas::client::{Client, Error, GetEntryOptions, DEFAULT_TIMEOUT};
use translaas::models::RequestContext;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn new_test_client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .build()
        .expect("client")
}

#[tokio::test]
async fn builder_validation_errors() {
    let err = Client::builder().build().expect_err("api key required");
    assert!(err.message.contains("ApiKey"));

    let err = Client::builder()
        .api_key("key")
        .base_url("not-a-url")
        .build()
        .expect_err("base url");
    assert!(err.message.contains("HTTP or HTTPS"));
}

#[tokio::test]
async fn builder_default_timeout() {
    let server = MockServer::start().await;
    let client = Client::builder()
        .api_key("key")
        .base_url(server.uri())
        .build()
        .unwrap();
    assert_eq!(client.timeout(), DEFAULT_TIMEOUT);
}

#[tokio::test]
async fn get_entry_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(header("X-Api-Key", "test-api-key"))
        .and(header("Accept", "text/plain"))
        .and(query_param("group", "ui"))
        .and(query_param("entry", "greeting"))
        .and(query_param("lang", "en"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", r#"W/"abc""#)
                .set_body_string("Hello, World!"),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_entry(
            "ui",
            "greeting",
            "en",
            GetEntryOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert_eq!(got, "Hello, World!");
    assert_eq!(ctx.response_etag.as_deref(), Some(r#"W/"abc""#));
    assert!(!ctx.not_modified);
}

#[tokio::test]
async fn get_entry_with_number() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("N", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_string("5 items"))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_entry("ui", "items", "en", GetEntryOptions::new().number(5.0))
        .await
        .unwrap();
    assert_eq!(got, "5 items");
}

#[tokio::test]
async fn get_entry_with_decimal_number() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("N", "1.31"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    client
        .get_entry("ui", "items", "en", GetEntryOptions::new().number(1.31))
        .await
        .unwrap();
}

#[tokio::test]
async fn get_entry_with_parameters() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("userName", "John Doe"))
        .respond_with(ResponseTemplate::new(200).set_body_string("hi"))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut params = HashMap::new();
    params.insert("userName".to_string(), "John Doe".to_string());
    client
        .get_entry(
            "ui",
            "greeting",
            "en",
            GetEntryOptions::new().parameters(params),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn get_entry_with_request_context() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("channel", "beta"))
        .and(query_param("v", "snap-1"))
        .and(query_param("project", "proj-1"))
        .and(header("If-None-Match", r#""etag-1""#))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext {
        channel: Some("beta".to_string()),
        version: Some("snap-1".to_string()),
        project: Some("proj-1".to_string()),
        if_none_match: Some(r#""etag-1""#.to_string()),
        ..Default::default()
    };
    client
        .get_entry(
            "ui",
            "greeting",
            "en",
            GetEntryOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn get_entry_default_project_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("project", "default-proj"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .default_project_id("default-proj")
        .build()
        .unwrap();
    client
        .get_entry("ui", "greeting", "en", GetEntryOptions::new())
        .await
        .unwrap();
}

#[tokio::test]
async fn get_entry_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_entry("ui", "missing.entry", "en", GetEntryOptions::new())
        .await
        .unwrap();
    assert_eq!(got, "missing.entry");
}

#[tokio::test]
async fn get_entry_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(ResponseTemplate::new(304).insert_header("ETag", r#"W/"xyz""#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_entry(
            "ui",
            "greeting",
            "en",
            GetEntryOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert_eq!(got, "");
    assert!(ctx.not_modified);
    assert_eq!(ctx.response_etag.as_deref(), Some(r#"W/"xyz""#));
}

#[tokio::test]
async fn get_entry_api_errors() {
    let cases = [
        (
            401,
            json!({"message":"invalid key","code":"AUTH"}).to_string(),
            401,
            Some("[AUTH] invalid key"),
        ),
        (
            500,
            "Internal Server Error".to_string(),
            500,
            Some("API request failed with status code 500."),
        ),
        (404, "Not Found".to_string(), 404, None),
    ];

    for (status, body, want_status, want_msg) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/sdk/v1/translations/text"))
            .respond_with(ResponseTemplate::new(status).set_body_string(body))
            .mount(&server)
            .await;

        let client = new_test_client(&server).await;
        let err = client
            .get_entry("ui", "entry", "en", GetEntryOptions::new())
            .await
            .expect_err("api error");
        let api = err.as_api().expect("Error::Api");
        assert_eq!(api.status_code, want_status);
        if let Some(msg) = want_msg {
            assert_eq!(api.message.as_deref(), Some(msg));
        }
    }
}

#[tokio::test]
async fn get_entry_validation_errors_skip_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("should not be called"))
        .expect(0)
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let err = client
        .get_entry("", "entry", "en", GetEntryOptions::new())
        .await
        .expect_err("group required");
    match err {
        Error::Configuration(cfg) => assert!(cfg.message.contains("group")),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn get_entry_timeout_maps_to_408() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("late")
                .set_delay(Duration::from_millis(400)),
        )
        .mount(&server)
        .await;

    let timeout = Duration::from_millis(50);
    let http = reqwest::Client::builder().timeout(timeout).build().unwrap();
    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .timeout(timeout)
        .http_client(http)
        .build()
        .unwrap();

    let err = client
        .get_entry("ui", "entry", "en", GetEntryOptions::new())
        .await
        .expect_err("timeout");
    let api = err.as_api().expect("Error::Api");
    assert_eq!(api.status_code, 408);
    assert!(api.message.as_deref().unwrap_or("").contains("timed out"));
}
