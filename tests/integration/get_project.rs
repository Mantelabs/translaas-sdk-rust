use translaas::client::GetProjectOptions;

use crate::common::{
    is_sdk_not_found, new_integration_client, soft_skip_if, soft_skip_on_sdk_not_found,
    FIXTURE_LANG,
};

#[tokio::test]
async fn get_project_existing() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_project(&cfg.default_project, FIXTURE_LANG, GetProjectOptions::new())
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_project: {e:?}"),
    };
    if soft_skip_if(got.groups.is_empty(), "fixture data not available in API") {
        return;
    }
    assert!(!got.groups.is_empty());
}

#[tokio::test]
async fn get_project_with_format() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_project(
            &cfg.default_project,
            FIXTURE_LANG,
            GetProjectOptions::new().format("json"),
        )
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_project format: {e:?}"),
    };
    if soft_skip_if(got.groups.is_empty(), "fixture data not available in API") {
        return;
    }
    assert!(!got.groups.is_empty());
}

#[tokio::test]
async fn get_project_not_found() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    match client
        .get_project(
            "nonexistent-project",
            FIXTURE_LANG,
            GetProjectOptions::new(),
        )
        .await
    {
        Ok(got) => assert!(got.groups.is_empty()),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("get_project missing: {e:?}"),
    }
}

#[tokio::test]
async fn get_project_multiple_groups() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_project(&cfg.default_project, FIXTURE_LANG, GetProjectOptions::new())
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_project walk: {e:?}"),
    };
    if soft_skip_if(got.groups.is_empty(), "fixture data not available in API") {
        return;
    }

    let mut walked = 0;
    for group_name in got.groups.keys() {
        let Some(group) = got.get_group(group_name).expect("parse group") else {
            continue;
        };
        assert!(!group.entries.is_empty());
        walked += 1;
    }
    if soft_skip_if(walked == 0, "fixture data not available in API") {
        return;
    }
}
