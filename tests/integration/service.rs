use translaas::client::ClientBuilder;
use translaas::service::{
    DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

use crate::common::{
    require_integration_config, soft_skip_if, FIXTURE_ENTRY_SAVE, FIXTURE_GROUP, FIXTURE_LANG,
};

#[tokio::test]
async fn service_t_explicit_language() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };

    let client = ClientBuilder::new()
        .api_key(&cfg.api_key)
        .base_url(&cfg.base_url)
        .default_project_id(&cfg.default_project)
        .build()
        .expect("client");

    let resolver =
        LanguageResolver::new([DefaultLanguageProvider::new(FIXTURE_LANG)]).expect("resolver");
    let service = Service::new(
        client,
        ServiceOptions {
            resolver: Some(resolver),
        },
    );

    let got = service
        .t(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            TOptions::new().lang(FIXTURE_LANG),
        )
        .await
        .expect("service t");
    if soft_skip_if(
        got == FIXTURE_ENTRY_SAVE,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}
