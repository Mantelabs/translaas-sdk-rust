//! Consumer-facing HTTP client boundary for decorator wrapping.

use crate::models::{
    OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem, TranslationGroup,
    TranslationProject, ValidateApiKeyResponse,
};

use super::{
    Client, Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions,
    GetProjectLocalesOptions, GetProjectOptions,
};

/// Live Translaas HTTP client surface for decorator wrapping (`cachefile` in issue #10).
#[allow(async_fn_in_trait)]
pub trait TranslaasClient: Send + Sync {
    /// Retrieves a single rendered translation string (plain text body).
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error>;

    /// Retrieves one translation group for a project and language.
    async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error>;

    /// Retrieves all translation groups for a project and language.
    async fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error>;

    /// Lists locales available for a project.
    async fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error>;

    /// Downloads the offline translation bundle as a ZIP archive.
    async fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error>;

    /// Reports missing translation keys (no-op when `keys` is empty).
    async fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error>;

    /// Validates the configured API key.
    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error>;
}

impl TranslaasClient for Client {
    async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        Client::get_entry(self, group, entry, lang, opts).await
    }

    async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        Client::get_group(self, project, group, lang, opts).await
    }

    async fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        Client::get_project(self, project, lang, opts).await
    }

    async fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        Client::get_project_locales(self, project, opts).await
    }

    async fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        Client::get_offline_cache(self, project, opts).await
    }

    async fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        Client::report_missing_keys(self, keys).await
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        Client::validate_api_key(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn client_is_send_sync() {
        assert_send_sync::<Client>();
    }
}
