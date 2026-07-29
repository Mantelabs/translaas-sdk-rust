//! Offline ZIP import into a file-backed cache directory.

use crate::models::OfflineCacheError;

use super::file_provider::FileProvider;
use super::paths::offline_cache_err;
use super::zip_bundle::{apply_offline_bundle, parse_offline_zip, resolve_project_key};

impl FileProvider {
    /// Parses `zip_bytes` and persists the matching project into this provider's cache directory.
    ///
    /// Uses [`super::Provider::save_locales`] and [`super::Provider::save_project`] so atomic
    /// writes and manifest updates stay consistent with API sync.
    pub fn import_offline_bundle(
        &self,
        project: &str,
        zip_bytes: &[u8],
    ) -> Result<(), OfflineCacheError> {
        let project = project.trim();
        if project.is_empty() {
            return Err(offline_cache_err(
                self.cache_directory(),
                project,
                "",
                "project must not be empty",
                None,
            ));
        }

        self.check_operation_cancelled(project, "")?;

        let bundle = parse_offline_zip(zip_bytes).map_err(|mut err| {
            err.cache_directory = Some(self.cache_directory().display().to_string());
            err.project = Some(project.to_string());
            err
        })?;

        let key = resolve_project_key(&bundle, project).map_err(|err| {
            offline_cache_err(self.cache_directory(), project, "", err.to_string(), None)
        })?;

        apply_offline_bundle(self, project, &key, &bundle)
    }
}
