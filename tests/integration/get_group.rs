use translaas::client::GetGroupOptions;

use crate::common::{
    is_sdk_not_found, new_integration_client, soft_skip_if, soft_skip_on_sdk_not_found,
    FIXTURE_GROUP, FIXTURE_LANG,
};

#[tokio::test]
async fn get_group_existing() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_group(
            &cfg.default_project,
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_group: {e:?}"),
    };
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

    let got = match client
        .get_group(
            &cfg.default_project,
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new().format("json"),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_group format: {e:?}"),
    };
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

    match client
        .get_group(
            &cfg.default_project,
            "nonexistent-group",
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
    {
        Ok(got) => assert!(got.entries.is_empty()),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("get_group missing group: {e:?}"),
    }
}

#[tokio::test]
async fn get_group_project_not_found() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    match client
        .get_group(
            "nonexistent-project",
            FIXTURE_GROUP,
            FIXTURE_LANG,
            GetGroupOptions::new(),
        )
        .await
    {
        Ok(got) => assert!(got.entries.is_empty()),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("get_group missing project: {e:?}"),
    }
}
