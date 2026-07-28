//! Integration tests for `translaas::axum`.

#![allow(clippy::type_complexity)]

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;
use translaas::axum::{middleware, translaas_middleware, MiddlewareOptions, Translaas};
use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    ConfigurationError, OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem,
    TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};
use translaas::service::{
    DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

#[derive(Default)]
struct MockClientState {
    last_lang: String,
}

struct MockClient {
    state: Arc<Mutex<MockClientState>>,
}

impl MockClient {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(MockClientState::default())),
        })
    }

    fn last_lang(&self) -> String {
        self.state.lock().unwrap().last_lang.clone()
    }
}

impl TranslaasClient for MockClient {
    async fn get_entry(
        &self,
        _: &str,
        _: &str,
        lang: &str,
        _: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        self.state.lock().unwrap().last_lang = lang.to_string();
        Ok("hello".to_string())
    }

    async fn get_group(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected get_group".into(),
        }))
    }

    async fn get_project(
        &self,
        _: &str,
        _: &str,
        _: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected get_project".into(),
        }))
    }

    async fn get_project_locales(
        &self,
        _: &str,
        _: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected get_project_locales".into(),
        }))
    }

    async fn get_offline_cache(
        &self,
        _: &str,
        _: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected get_offline_cache".into(),
        }))
    }

    async fn report_missing_keys(&self, _: &[ReportMissingKeyItem]) -> Result<(), Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected report_missing_keys".into(),
        }))
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        Err(Error::Configuration(ConfigurationError {
            message: "unexpected validate_api_key".into(),
        }))
    }
}

#[derive(Clone)]
struct SharedMockClient(Arc<MockClient>);

impl SharedMockClient {
    fn new() -> Self {
        Self(MockClient::new())
    }

    fn last_lang(&self) -> String {
        self.0.last_lang()
    }
}

impl TranslaasClient for SharedMockClient {
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

fn base_service(client: SharedMockClient) -> Service<SharedMockClient> {
    let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")]).expect("resolver");
    Service::new(
        client,
        ServiceOptions {
            resolver: Some(resolver),
        },
    )
}

fn app_with_middleware(client: SharedMockClient) -> Router {
    let state = Arc::new(
        middleware(MiddlewareOptions::with_base_service(base_service(
            client.clone(),
        )))
        .expect("middleware"),
    );

    Router::new()
        .route("/", get(handler))
        .layer(from_fn_with_state(state.clone(), translaas_middleware))
        .with_state(state)
}

async fn handler(Translaas(service): Translaas<SharedMockClient>) -> String {
    service
        .t("ui", "welcome", TOptions::new())
        .await
        .expect("translation")
}

async fn handler_with_explicit_lang(Translaas(service): Translaas<SharedMockClient>) -> String {
    service
        .t("ui", "welcome", TOptions::new().lang("pt"))
        .await
        .expect("translation")
}

#[tokio::test]
async fn middleware_injects_service_into_extensions() {
    let client = SharedMockClient::new();
    let app = app_with_middleware(client);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/?lang=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn middleware_resolves_query_language_on_t() {
    let client = SharedMockClient::new();
    let app = app_with_middleware(client.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/?lang=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(client.last_lang(), "de");
    let _ = response.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn middleware_accept_language_fallback() {
    let client = SharedMockClient::new();
    let app = app_with_middleware(client.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Accept-Language", "fr-FR,fr;q=0.9")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(client.last_lang(), "fr");
    let _ = response.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn service_from_extensions_missing() {
    let app = Router::new().route("/", get(handler));

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn middleware_prepended_provider_does_not_mutate_base() {
    let client = SharedMockClient::new();
    let app = app_with_middleware(client.clone());

    let first = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/?lang=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(client.last_lang(), "de");
    let _ = first.into_body().collect().await.unwrap();

    let second = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(client.last_lang(), "en");
    let _ = second.into_body().collect().await.unwrap();
}

#[tokio::test]
async fn axum_path_explicit_lang_bypasses_request_language() {
    let client = SharedMockClient::new();
    let state = Arc::new(
        middleware(MiddlewareOptions::with_base_service(base_service(
            client.clone(),
        )))
        .expect("middleware"),
    );

    let app = Router::new()
        .route("/", get(handler_with_explicit_lang))
        .layer(from_fn_with_state(state.clone(), translaas_middleware))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/?lang=de")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(client.last_lang(), "pt");
    let _ = response.into_body().collect().await.unwrap();
}

#[test]
fn middleware_requires_base_service() {
    let result = middleware(MiddlewareOptions::<SharedMockClient> {
        base_service: None,
        language_sources: None,
        request_language: translaas::axum::RequestLanguageOptions::default(),
        route_language: None,
    });

    assert!(matches!(
        result,
        Err(translaas::axum::MiddlewareError::MissingBaseService)
    ));
}
