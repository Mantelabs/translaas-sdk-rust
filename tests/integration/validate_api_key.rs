use translaas::client::GetEntryOptions;
use translaas::models::read_json_ulid;

use crate::common::{
    integration_client_builder, require_integration_config, soft_skip_if, soft_skip_on_sdk_not_found,
    FIXTURE_ENTRY_SAVE, FIXTURE_GROUP, FIXTURE_LANG,
};

#[tokio::test]
async fn validate_api_key_valid() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };
    let client = integration_client_builder(&cfg, std::time::Duration::from_secs(30))
        .api_key(&cfg.api_key)
        .base_url(&cfg.base_url)
        .build()
        .expect("client");

    let got = client.validate_api_key().await.expect("validate_api_key");
    assert!(got.is_valid);
}

#[tokio::test]
async fn build_with_resolved_project_single_project_key() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };

    let client = integration_client_builder(&cfg, std::time::Duration::from_secs(30))
        .api_key(&cfg.api_key)
        .base_url(&cfg.base_url)
        .build_with_resolved_project()
        .await
        .expect("build_with_resolved_project");

    let validate = client.validate_api_key().await.expect("validate");
    let project_id = validate.project_id.as_ref();
    if read_json_ulid(project_id.unwrap_or(&serde_json::Value::Null)).is_none() {
        eprintln!("skipping: API key is not single-project scoped");
        return;
    }

    let got = match client
        .get_entry(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            FIXTURE_LANG,
            GetEntryOptions::new(),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_entry resolved project: {e:?}"),
    };
    if soft_skip_if(
        got == FIXTURE_ENTRY_SAVE,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}
