//! Wiremock coverage for write/validate endpoints and bootstrap (issue #5).

use translaas::client::{Client, TranslaasClient};
use translaas::models::ReportMissingKeyItem;
use wiremock::matchers::{header, method, path};
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
async fn report_missing_keys_empty_no_op() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sdk/v1/translations/report-missing"))
        .respond_with(ResponseTemplate::new(202))
        .expect(0)
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    client.report_missing_keys(&[]).await.unwrap();
}

#[tokio::test]
async fn report_missing_keys_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sdk/v1/translations/report-missing"))
        .and(header("X-Api-Key", "test-api-key"))
        .and(header("Content-Type", "application/json"))
        .and(wiremock::matchers::body_json(serde_json::json!({
            "keys": [{
                "groupKey": "g",
                "entryKey": "k",
                "languageIsoCode": "en"
            }]
        })))
        .respond_with(ResponseTemplate::new(202))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    client
        .report_missing_keys(&[ReportMissingKeyItem {
            group_key: "g".to_string(),
            entry_key: "k".to_string(),
            language_iso_code: "en".to_string(),
        }])
        .await
        .unwrap();
}

#[tokio::test]
async fn report_missing_keys_validation_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sdk/v1/translations/report-missing"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let err = client
        .report_missing_keys(&[ReportMissingKeyItem {
            group_key: "g".to_string(),
            entry_key: "k".to_string(),
            language_iso_code: "en".to_string(),
        }])
        .await
        .expect_err("400");
    assert_eq!(err.as_api().unwrap().status_code, 400);
}

#[tokio::test]
async fn report_missing_keys_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sdk/v1/translations/report-missing"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let err = client
        .report_missing_keys(&[ReportMissingKeyItem {
            group_key: "g".to_string(),
            entry_key: "k".to_string(),
            language_iso_code: "en".to_string(),
        }])
        .await
        .expect_err("401");
    assert_eq!(err.as_api().unwrap().status_code, 401);
}

#[tokio::test]
async fn validate_api_key_success() {
    let server = MockServer::start().await;
    let fixture = load_testdata("validate_api_key_tenant.json");
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .and(header("X-Api-Key", "test-api-key"))
        .and(header("Accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let got = client.validate_api_key().await.unwrap();
    assert!(got.is_valid);
    assert_eq!(got.project_ids.as_ref().map(Vec::len), Some(2));
}

#[tokio::test]
async fn validate_api_key_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let err = client.validate_api_key().await.expect_err("401");
    assert_eq!(err.as_api().unwrap().status_code, 401);
}

#[tokio::test]
async fn build_with_resolved_project_single_project_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"isValid":true,"projectId":"01PROJECTULID123456789012"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .build_with_resolved_project()
        .await
        .unwrap();
    assert_eq!(
        client.default_project_id(),
        Some("01PROJECTULID123456789012")
    );
}

#[tokio::test]
async fn build_with_resolved_project_tenant_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"isValid":true,"tenantId":"01TENANT"}"#),
        )
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .build_with_resolved_project()
        .await
        .unwrap();
    assert!(client.default_project_id().is_none());
}

#[tokio::test]
async fn build_with_resolved_project_preconfigured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/api-keys/validate"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let client = Client::builder()
        .api_key("test-api-key")
        .base_url(server.uri())
        .default_project_id("preset")
        .build_with_resolved_project()
        .await
        .unwrap();
    assert_eq!(client.default_project_id(), Some("preset"));
}

#[tokio::test]
async fn translaas_client_trait_delegates_to_client() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/locales"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"locales":["en"]}"#))
        .mount(&server)
        .await;

    let client = new_test_client(&server).await;
    let locales = TranslaasClient::get_project_locales(
        &client,
        "my-app",
        translaas::client::GetProjectLocalesOptions::new(),
    )
    .await
    .unwrap();
    assert_eq!(locales.locales, vec!["en"]);
}
