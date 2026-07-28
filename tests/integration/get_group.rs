use translaas::client::GetGroupOptions;

use crate::common::{new_integration_client, soft_skip_if, FIXTURE_GROUP, FIXTURE_LANG};

#[tokio::test]
async fn get_group_existing() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_group(
            &cfg.default_project,
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
        .expect("get_group");
    if soft_skip_if(got.entries.is_empty(), "fixture data not available in API") {
        return;
    }
    assert!(!got.entries.is_empty());
}

#[tokio::test]
async fn get_group_with_format() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_group(
            &cfg.default_project,
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new().format("json"),
        )
        .await
        .expect("get_group format");
    if soft_skip_if(got.entries.is_empty(), "fixture data not available in API") {
        return;
    }
    assert!(!got.entries.is_empty());
}

#[tokio::test]
async fn get_group_not_found() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_group(
            &cfg.default_project,
            "nonexistent-group",
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
        .expect("get_group missing group");
    assert!(got.entries.is_empty());
}

#[tokio::test]
async fn get_group_project_not_found() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_group(
            "nonexistent-project",
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
        .expect("get_group missing project");
    assert!(got.entries.is_empty());
}
