//! Best-effort group cache warming after successful API reads.

use std::collections::HashMap;

use crate::client::{Error, GetGroupOptions, TranslaasClient};
use serde_json::Value;

use super::provider::{Provider, SaveOptions};

/// Fetches the group from the inner client and merges its entries into the cached project.
pub(crate) async fn update_group_cache<C, P>(
    inner: &C,
    cache: &P,
    project: &str,
    group: &str,
    lang: &str,
) -> Result<(), Error>
where
    C: TranslaasClient,
    P: Provider + ?Sized,
{
    let group_data = inner
        .get_group(project, group, lang, GetGroupOptions::new())
        .await?;
    merge_group_into_project(cache, project, group, lang, &group_data.entries)
}

/// Swallows warm failures so a successful API read is not lost.
pub(crate) async fn try_update_group_cache<C, P>(
    inner: &C,
    cache: &P,
    project: &str,
    group: &str,
    lang: &str,
) where
    C: TranslaasClient,
    P: Provider + ?Sized,
{
    let _ = update_group_cache(inner, cache, project, group, lang).await;
}

fn merge_group_into_project<P: Provider + ?Sized>(
    cache: &P,
    project: &str,
    group: &str,
    lang: &str,
    entries: &HashMap<String, Value>,
) -> Result<(), Error> {
    let entries_json =
        serde_json::to_value(entries).map_err(|err| map_offline_io_error(project, lang, err))?;

    let mut project_to_save = cache
        .get_project(project, lang)
        .map_err(map_offline_error)?
        .unwrap_or_default();

    if project_to_save.groups.is_empty() {
        project_to_save.groups = HashMap::new();
    }
    project_to_save
        .groups
        .insert(group.to_string(), entries_json);

    cache
        .save_project(project, lang, &project_to_save, SaveOptions::new())
        .map_err(map_offline_error)?;

    Ok(())
}

fn map_offline_error(err: crate::models::OfflineCacheError) -> Error {
    Error::from(err)
}

fn map_offline_io_error(project: &str, lang: &str, err: serde_json::Error) -> Error {
    Error::from(crate::models::OfflineCacheError::new(
        err.to_string(),
        None,
        Some(project.to_string()),
        Some(lang.to_string()),
        Some(Box::new(err)),
    ))
}
