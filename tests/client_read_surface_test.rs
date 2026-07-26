//! Wiremock coverage for JSON read endpoints (issue #5).

use translaas::client::{
    Client, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions, GetProjectOptions,
};
use translaas::models::RequestContext;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn load_testdata(name: &str) -> String {
    std::fs::read_to_string(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|err| panic!("read testdata {name}: {err}"))
}

async fn new_test_client(server: &MockServer) -> Client {
    Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .build()
        .expect("client")
}

#[tokio::test]
async fn get_group_success() {
    let server = MockServer::start().await;
    let fixture = load_testdata("translation_group_full_api.json");
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .and(header("X-Api-Key", "test-api-key"))
        .and(header("Accept", "application/json"))
        .and(query_param("project", "my-project"))
        .and(query_param("group", "ui"))
        .and(query_param("lang", "en"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", r#"W/"grp-etag""#)
                .set_body_string(fixture),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_group(
            "my-project",
            "ui",
            "en",
            GetGroupOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert_eq!(got.project.as_deref(), Some("my-project"));
    assert_eq!(got.get_value("welcome"), Some("Welcome"));
    assert_eq!(ctx.response_etag.as_deref(), Some(r#"W/"grp-etag""#));
}

#[tokio::test]
async fn get_group_flat_format() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .and(query_param("format", "flat-json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"title":"Checkout"}"#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_group("p", "g", "en", GetGroupOptions::new().format("flat-json"))
        .await
        .unwrap();
    assert_eq!(got.get_value("title"), Some("Checkout"));
}

#[tokio::test]
async fn get_group_with_request_context() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .and(query_param("channel", "canary"))
        .and(query_param("v", "42"))
        .and(query_param("includeContext", "true"))
        .and(header("If-None-Match", r#"W/"old""#))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", r#"W/"grp-new""#)
                .set_body_string(r#"{"Entries":{"k":"v"}}"#),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext {
        channel: Some("canary".to_string()),
        version: Some("42".to_string()),
        include_context: Some(true),
        if_none_match: Some(r#"W/"old""#.to_string()),
        ..Default::default()
    };
    client
        .get_group(
            "p",
            "g",
            "en",
            GetGroupOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert_eq!(ctx.response_etag.as_deref(), Some(r#"W/"grp-new""#));
}

#[tokio::test]
async fn get_group_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_group("p", "g", "en", GetGroupOptions::new())
        .await
        .unwrap();
    assert!(got.entries.is_empty());
}

#[tokio::test]
async fn get_group_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(304).insert_header("ETag", r#"W/"new""#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext {
        if_none_match: Some(r#"W/"old""#.to_string()),
        ..Default::default()
    };
    let got = client
        .get_group(
            "p",
            "g",
            "en",
            GetGroupOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert!(ctx.not_modified);
    assert_eq!(ctx.response_etag.as_deref(), Some(r#"W/"new""#));
    assert!(got.entries.is_empty());
}

#[tokio::test]
async fn get_group_not_found() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_string(r#"{"message":"not found","code":"NOT_FOUND"}"#),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let err = client
        .get_group("p", "g", "en", GetGroupOptions::new())
        .await
        .expect_err("404");
    assert_eq!(err.as_api().unwrap().status_code, 404);
}

#[tokio::test]
async fn get_group_required_fields_skip_http() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    assert!(client
        .get_group("", "g", "en", GetGroupOptions::new())
        .await
        .is_err());
    assert!(client
        .get_group("p", "", "en", GetGroupOptions::new())
        .await
        .is_err());
}

#[tokio::test]
async fn get_project_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/project"))
        .and(query_param("project", "my-app"))
        .and(query_param("lang", "en"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"common":{"hello":"Hello"}}"#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_project("my-app", "en", GetProjectOptions::new())
        .await
        .unwrap();
    let group = got.get_group("common").unwrap().unwrap();
    assert_eq!(group.get_value("hello"), Some("Hello"));
}

#[tokio::test]
async fn get_project_with_options() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/project"))
        .and(query_param("format", "flat-json"))
        .and(query_param("channel", "stable"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext {
        channel: Some("stable".to_string()),
        version: Some("1".to_string()),
        include_context: Some(true),
        ..Default::default()
    };
    let got = client
        .get_project(
            "p",
            "en",
            GetProjectOptions::new()
                .format("flat-json")
                .request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert!(got.groups.is_empty());
}

#[tokio::test]
async fn get_project_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/project"))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_project(
            "p",
            "en",
            GetProjectOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert!(ctx.not_modified);
    assert!(got.groups.is_empty());
}

#[tokio::test]
async fn get_project_locales_success() {
    let server = MockServer::start().await;
    let fixture = load_testdata("project_locales.json");
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/locales"))
        .and(query_param("project", "my-app"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_project_locales("my-app", GetProjectLocalesOptions::new())
        .await
        .unwrap();
    assert_eq!(got.locales.len(), 4);
}

#[tokio::test]
async fn get_project_locales_no_content() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/locales"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_project_locales("my-app", GetProjectLocalesOptions::new())
        .await
        .unwrap();
    assert!(got.locales.is_empty());
}

#[tokio::test]
async fn get_project_locales_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/locales"))
        .respond_with(ResponseTemplate::new(304))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_project_locales(
            "my-app",
            GetProjectLocalesOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert!(ctx.not_modified);
    assert!(got.locales.is_empty());
}

#[tokio::test]
async fn get_offline_cache_success() {
    let server = MockServer::start().await;
    let zip_bytes = b"PK\x03\x04fake-zip";
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/offline-cache"))
        .and(header("Accept", "application/zip"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Content-Disposition",
                    r#"attachment; filename="bundle.zip""#,
                )
                .insert_header("ETag", r#"W/"offline-etag""#)
                .set_body_bytes(zip_bytes.to_vec()),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_offline_cache("my-app", GetOfflineCacheOptions::new())
        .await
        .unwrap();
    assert_eq!(got.content.as_deref(), Some(zip_bytes.as_slice()));
    assert_eq!(got.suggested_file_name.as_deref(), Some("bundle.zip"));
    assert_eq!(got.etag.as_deref(), Some(r#"W/"offline-etag""#));
}

#[tokio::test]
async fn get_offline_cache_filename_star() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/offline-cache"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header(
                    "Content-Disposition",
                    r#"attachment; filename*=UTF-8''my%20bundle.zip"#,
                )
                .set_body_string("zip"),
        )
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client
        .get_offline_cache("my-app", GetOfflineCacheOptions::new())
        .await
        .unwrap();
    assert_eq!(got.suggested_file_name.as_deref(), Some("my bundle.zip"));
}

#[tokio::test]
async fn get_offline_cache_not_modified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/offline-cache"))
        .respond_with(ResponseTemplate::new(304).insert_header("ETag", r#"W/"offline""#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let mut ctx = RequestContext::default();
    let got = client
        .get_offline_cache(
            "my-app",
            GetOfflineCacheOptions::new().request_context(&mut ctx),
        )
        .await
        .unwrap();
    assert!(got.not_modified);
    assert!(got.content.is_none());
    assert_eq!(got.etag.as_deref(), Some(r#"W/"offline""#));
    assert!(ctx.not_modified);
}

#[tokio::test]
async fn get_group_timeout_maps_to_408() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/group"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(200)))
        .mount(&server)
        .await;

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(50))
        .build()
        .unwrap();
    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .http_client(http_client)
        .build()
        .unwrap();

    let err = client
        .get_group("p", "g", "en", GetGroupOptions::new())
        .await
        .expect_err("timeout");
    assert_eq!(err.as_api().unwrap().status_code, 408);
}
