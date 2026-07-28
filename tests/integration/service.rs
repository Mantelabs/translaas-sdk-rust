use translaas::service::{
    DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
};

use crate::common::{
    integration_client_builder, require_integration_config, soft_skip_if,
    soft_skip_on_service_sdk_not_found, FIXTURE_ENTRY_SAVE, FIXTURE_GROUP, FIXTURE_LANG,
};

#[tokio::test]
async fn service_t_explicit_language() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };

    let client = integration_client_builder(&cfg, std::time::Duration::from_secs(30))
        .api_key(&cfg.api_key)
        .base_url(&cfg.base_url)
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

    let got = match service
        .t(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            TOptions::new().lang(FIXTURE_LANG),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_service_sdk_not_found(&e) => return,
        Err(e) => panic!("service t: {e:?}"),
    };
    if soft_skip_if(
        got == FIXTURE_ENTRY_SAVE,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}
