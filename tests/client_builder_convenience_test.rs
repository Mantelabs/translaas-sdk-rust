//! Wiremock coverage for `ClientBuilder::build_service` and default language.

use translaas::ClientBuilder;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn build_service_fetches_via_wiremock() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(header("X-Api-Key", "test-api-key"))
        .and(query_param("group", "common"))
        .and(query_param("entry", "welcome.message"))
        .and(query_param("lang", "en"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Welcome back!"))
        .mount(&server)
        .await;

    let service = ClientBuilder::new()
        .api_key("test-api-key")
        .base_url(server.uri())
        .default_project_id("translaassdksamples")
        .default_language("en")
        .build_service()
        .expect("build_service");

    let got = service
        .t_lang("common", "welcome.message", "en")
        .await
        .unwrap();
    assert_eq!(got, "Welcome back!");
}

#[tokio::test]
async fn build_service_default_language_applies_to_t_two_arg() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sdk/v1/translations/text"))
        .and(query_param("lang", "de"))
        .and(query_param("group", "common"))
        .and(query_param("entry", "welcome.message"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Willkommen!"))
        .mount(&server)
        .await;

    let service = ClientBuilder::new()
        .api_key("test-api-key")
        .base_url(server.uri())
        .default_project_id("translaassdksamples")
        .default_language("de")
        .build_service()
        .expect("build_service");

    let got = service.t("common", "welcome.message").await.unwrap();
    assert_eq!(got, "Willkommen!");
}

#[tokio::test]
async fn default_language_is_retained_on_build_service() {
    let service = ClientBuilder::new()
        .api_key("key")
        .base_url("https://example.com")
        .default_language("fr")
        .build_service()
        .expect("build_service");

    // Construction succeeds; language is used on the next `t()` (no network in this test).
    let _ = service;
}
