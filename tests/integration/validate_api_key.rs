use translaas::client::{ClientBuilder, GetEntryOptions};

use crate::common::{
    new_integration_client, require_integration_config, soft_skip_if, FIXTURE_ENTRY_SAVE,
    FIXTURE_GROUP, FIXTURE_LANG,
};
use translaas::models::read_json_ulid;

#[tokio::test]
async fn validate_api_key_valid() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    let got = client.validate_api_key().await.expect("validate_api_key");
    assert!(got.is_valid);
}

#[tokio::test]
async fn build_with_resolved_project_single_project_key() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };

    let client = ClientBuilder::new()
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

    let got = client
        .get_entry(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            FIXTURE_LANG,
            GetEntryOptions::new(),
        )
        .await
        .expect("get_entry resolved project");
    if soft_skip_if(
        got == FIXTURE_ENTRY_SAVE,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}
