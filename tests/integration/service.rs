use translaas::service::Service;

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
        .default_language(FIXTURE_LANG)
        .build()
        .expect("client");

    let service = Service::new(client);

    let got = match service
        .t_lang(FIXTURE_GROUP, FIXTURE_ENTRY_SAVE, FIXTURE_LANG)
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
