use translaas::client::GetProjectLocalesOptions;

use crate::common::{
    is_sdk_not_found, new_integration_client, soft_skip_if, soft_skip_on_sdk_not_found,
};

#[tokio::test]
async fn get_project_locales_existing() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_project_locales(&cfg.default_project, GetProjectLocalesOptions::new())
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_project_locales: {e:?}"),
    };
    if soft_skip_if(got.locales.is_empty(), "fixture data not available in API") {
        return;
    }
    assert!(!got.locales.is_empty());
}

#[tokio::test]
async fn get_project_locales_common() {
    let Some((cfg, client)) = new_integration_client().await else {
        return;
    };

    let got = match client
        .get_project_locales(&cfg.default_project, GetProjectLocalesOptions::new())
        .await
    {
        Ok(v) => v,
        Err(e) if soft_skip_on_sdk_not_found(&e) => return,
        Err(e) => panic!("get_project_locales common: {e:?}"),
    };
    if soft_skip_if(got.locales.is_empty(), "fixture data not available in API") {
        return;
    }

    const COMMON: [&str; 4] = ["en", "fr", "es", "de"];
    let found = got
        .locales
        .iter()
        .any(|locale| COMMON.contains(&locale.as_str()));
    if soft_skip_if(!found, "expected at least one common locale in fixture API") {
        return;
    }
    assert!(found);
}

#[tokio::test]
async fn get_project_locales_not_found() {
    let Some((_, client)) = new_integration_client().await else {
        return;
    };

    match client
        .get_project_locales("nonexistent-project", GetProjectLocalesOptions::new())
        .await
    {
        Ok(got) => assert!(got.locales.is_empty()),
        Err(e) if is_sdk_not_found(&e) => {}
        Err(e) => panic!("get_project_locales missing: {e:?}"),
    }
}
