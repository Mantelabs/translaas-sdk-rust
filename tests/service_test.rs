//! Integration tests for `service::Service` and language resolution.

#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use translaas::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use translaas::models::{
    ConfigurationError, OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem,
    RequestContext, TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};
use translaas::service::{
    ContextLanguageProvider, DefaultLanguageProvider, Error as ServiceError, LanguageContext,
    LanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

#[derive(Default)]
struct MockClientState {
    get_entry_calls: u32,
    last_lang: String,
    last_number: Option<f64>,
    last_parameters: HashMap<String, String>,
    last_request_context_project: Option<String>,
}

struct MockClient {
    state: Arc<Mutex<MockClientState>>,
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
}

impl MockClient {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(MockClientState::default())),
            get_entry_fn: Some(Box::new(|_, _, _, _| {
                Box::pin(async { Ok("hello".to_string()) })
            })),
        })
    }

    fn get_entry_calls(&self) -> u32 {
        self.state.lock().unwrap().get_entry_calls
    }

    fn last_lang(&self) -> String {
        self.state.lock().unwrap().last_lang.clone()
    }

    fn last_number(&self) -> Option<f64> {
        self.state.lock().unwrap().last_number
    }

    fn last_parameters(&self) -> HashMap<String, String> {
        self.state.lock().unwrap().last_parameters.clone()
    }

    fn last_request_context_project(&self) -> Option<String> {
        self.state
            .lock()
            .unwrap()
            .last_request_context_project
            .clone()
    }
}

impl TranslaasClient for MockClient {
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        {
            let mut state = self.state.lock().unwrap();
            state.get_entry_calls += 1;
            state.last_lang = lang.to_string();
            state.last_number = opts.number;
            state.last_parameters = opts.parameters.clone();
            state.last_request_context_project = opts
                .request_context
                .as_ref()
                .map(|ctx| ctx.project.clone().unwrap_or_default());
        }

        if let Some(ref handler) = self.get_entry_fn {
            return handler(group, entry, lang, opts).await;
        }
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

    fn get_entry_calls(&self) -> u32 {
        self.0.get_entry_calls()
    }

    fn last_lang(&self) -> String {
        self.0.last_lang()
    }

    fn last_number(&self) -> Option<f64> {
        self.0.last_number()
    }

    fn last_parameters(&self) -> HashMap<String, String> {
        self.0.last_parameters()
    }

    fn last_request_context_project(&self) -> Option<String> {
        self.0.last_request_context_project()
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

fn assert_no_language(err: ServiceError) {
    assert!(
        err.is_no_language(),
        "expected NoLanguageError, got {err:?}"
    );
}

#[tokio::test]
async fn t_explicit_lang_bypasses_resolver() {
    let inner = SharedMockClient::new();
    let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")]).unwrap();
    let service = Service::new(
        inner.clone(),
        ServiceOptions {
            resolver: Some(resolver),
        },
    );

    let got = service
        .t("common", "welcome", TOptions::new().lang("de"))
        .await
        .unwrap();

    assert_eq!(got, "hello");
    assert_eq!(inner.last_lang(), "de");
}

#[tokio::test]
async fn t_empty_lang_uses_resolver() {
    let inner = SharedMockClient::new();
    let resolver = LanguageResolver::new([DefaultLanguageProvider::new("es")]).unwrap();
    let service = Service::new(
        inner.clone(),
        ServiceOptions {
            resolver: Some(resolver),
        },
    );

    service
        .t("common", "welcome", TOptions::new().lang("   "))
        .await
        .unwrap();

    assert_eq!(inner.last_lang(), "es");
}

#[tokio::test]
async fn t_no_resolver_no_lang() {
    let inner = SharedMockClient::new();
    let service = Service::new(inner.clone(), ServiceOptions::default());

    let err = service
        .t("common", "welcome", TOptions::new())
        .await
        .unwrap_err();

    assert_no_language(err);
    assert_eq!(inner.get_entry_calls(), 0);
}

#[tokio::test]
async fn t_forwards_get_entry_options() {
    let inner = SharedMockClient::new();
    let service = Service::new(inner.clone(), ServiceOptions::default());

    let mut request_context = RequestContext {
        project: Some("demo".into()),
        ..RequestContext::default()
    };
    let mut params = HashMap::new();
    params.insert("name".into(), "Ada".into());

    service
        .t(
            "common",
            "items",
            TOptions::new()
                .lang("en")
                .number(3.0)
                .parameters(params)
                .request_context(&mut request_context),
        )
        .await
        .unwrap();

    assert_eq!(inner.last_number(), Some(3.0));
    assert_eq!(
        inner.last_parameters().get("name").map(String::as_str),
        Some("Ada")
    );
    assert_eq!(
        inner.last_request_context_project().as_deref(),
        Some("demo")
    );
}

#[tokio::test]
async fn t_resolver_from_context() {
    let inner = SharedMockClient::new();
    let resolver = LanguageResolver::from_providers(vec![
        Arc::new(ContextLanguageProvider) as Arc<dyn LanguageProvider>,
        Arc::new(DefaultLanguageProvider::new("en")),
    ])
    .unwrap();
    let service = Service::new(
        inner.clone(),
        ServiceOptions {
            resolver: Some(resolver),
        },
    );

    service
        .t(
            "common",
            "welcome",
            TOptions::new().language_context(LanguageContext::new().with_language("pt")),
        )
        .await
        .unwrap();

    assert_eq!(inner.last_lang(), "pt");
}

#[tokio::test]
async fn with_prepended_providers() {
    let inner = SharedMockClient::new();
    let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")]).unwrap();
    let base = Service::new(
        inner.clone(),
        ServiceOptions {
            resolver: Some(resolver),
        },
    );

    let scoped = base
        .with_prepended_providers([DefaultLanguageProvider::new("de")])
        .unwrap();

    scoped
        .t("common", "welcome", TOptions::new())
        .await
        .unwrap();
    assert_eq!(inner.last_lang(), "de");

    base.t("common", "welcome", TOptions::new()).await.unwrap();
    assert_eq!(inner.last_lang(), "en");
}
