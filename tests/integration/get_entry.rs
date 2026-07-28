use translaas::client::GetEntryOptions;

use crate::common::{
    is_sdk_not_found, new_client_with_options, new_integration_client, require_integration_config,
    soft_skip_if, soft_skip_on_sdk_not_found, FIXTURE_ENTRY_PLURAL, FIXTURE_ENTRY_SAVE,
    FIXTURE_GROUP, FIXTURE_GROUP_MESSAGES, FIXTURE_LANG,
};

#[tokio::test]
async fn get_entry_existing() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

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
        Err(e) => panic!("get_entry: {e:?}"),
    };
    if soft_skip_if(
        got.is_empty() || got == FIXTURE_ENTRY_SAVE,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}

#[tokio::test]
async fn get_entry_with_pluralization() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_entry(
            FIXTURE_GROUP_MESSAGES,
            FIXTURE_ENTRY_PLURAL,
            FIXTURE_LANG,
            GetEntryOptions::new().number(5.0),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_entry plural: {e:?}"),
    };
    if soft_skip_if(
        got.is_empty() || got == FIXTURE_ENTRY_PLURAL,
        "fixture data not available in API",
    ) {
        return;
    }
    assert!(!got.is_empty());
}

#[tokio::test]
async fn get_entry_not_found_returns_entry_key() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    const ENTRY: &str = "nonexistent.entry";
    match client
        .get_entry("nonexistent", ENTRY, FIXTURE_LANG, GetEntryOptions::new())
        .await
    {
        Ok(got) => assert_eq!(got, ENTRY),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("get_entry not found: {e:?}"),
    }
}

#[tokio::test]
async fn get_entry_invalid_api_key() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };
    let client = new_client_with_options(
        &cfg,
        "invalid-api-key",
        &cfg.base_url,
        std::time::Duration::from_secs(30),
    );

    let err = client
        .get_entry(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            FIXTURE_LANG,
            GetEntryOptions::new(),
        )
        .await
        .expect_err("invalid api key");
    let api = err.as_api().expect("Error::Api");
    assert!(api.status_code == 401 || api.status_code == 403);
}
