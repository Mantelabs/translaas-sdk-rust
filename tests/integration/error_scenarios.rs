use std::time::Duration;

use translaas::client::GetEntryOptions;

use crate::common::{
    is_sdk_not_found, new_client_with_options, new_integration_client, require_integration_config,
    FIXTURE_ENTRY_SAVE, FIXTURE_GROUP, FIXTURE_LANG,
};

#[tokio::test]
async fn error_invalid_api_key() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };
    let client = new_client_with_options(
        &cfg,
        "invalid-api-key-12345",
        &cfg.base_url,
        Duration::from_secs(30),
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

#[tokio::test]
async fn error_invalid_base_url() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };
    let client = new_client_with_options(
        &cfg,
        &cfg.api_key,
        "https://invalid-url-that-does-not-exist-12345.com",
        Duration::from_secs(30),
    );

    let err = client
        .get_entry(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            FIXTURE_LANG,
            GetEntryOptions::new(),
        )
        .await
        .expect_err("invalid base url");
    let _ = err;
}

#[tokio::test]
async fn error_request_timeout() {
    let Some(cfg) = require_integration_config().await else {
        return;
    };
    let client =
        new_client_with_options(&cfg, &cfg.api_key, &cfg.base_url, Duration::from_millis(1));

    let err = client
        .get_entry(
            FIXTURE_GROUP,
            FIXTURE_ENTRY_SAVE,
            FIXTURE_LANG,
            GetEntryOptions::new(),
        )
        .await
        .expect_err("timeout");
    let api = err.as_api().expect("Error::Api timeout");
    assert_eq!(api.status_code, 408);
    assert!(api
        .message
        .as_deref()
        .unwrap_or("")
        .to_lowercase()
        .contains("timed out"));
}

#[tokio::test]
async fn error_entry_not_found_returns_key() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    const ENTRY: &str = "nonexistent-entry";
    match client
        .get_entry(
            "nonexistent-group",
            ENTRY,
            "nonexistent-lang",
            GetEntryOptions::new(),
        )
        .await
    {
        Ok(got) => assert_eq!(got, ENTRY),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("entry not found: {e:?}"),
    }
}
