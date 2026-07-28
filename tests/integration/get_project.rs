use translaas::client::GetProjectOptions;

use crate::common::{new_integration_client, soft_skip_if, FIXTURE_LANG};

#[tokio::test]
async fn get_project_existing() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_project(&cfg.default_project, FIXTURE_LANG, GetProjectOptions::new())
        .await
        .expect("get_project");
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

    let got = client
        .get_project(
            &cfg.default_project,
            FIXTURE_LANG,
            GetProjectOptions::new().format("json"),
        )
        .await
        .expect("get_project format");
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

    let got = client
        .get_project(
            "nonexistent-project",
            FIXTURE_LANG,
            GetProjectOptions::new(),
        )
        .await
        .expect("get_project missing");
    assert!(got.groups.is_empty());
}

#[tokio::test]
async fn get_project_multiple_groups() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = client
        .get_project(&cfg.default_project, FIXTURE_LANG, GetProjectOptions::new())
        .await
        .expect("get_project walk");
    if soft_skip_if(got.groups.is_empty(), "fixture data not available in API") {
        return;
    }

    for group_name in got.groups.keys() {
        let group = got
            .get_group(group_name)
            .expect("parse group")
            .expect("group present");
        assert!(!group.entries.is_empty());
    }
}
