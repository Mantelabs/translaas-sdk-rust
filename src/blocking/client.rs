//! Blocking wrapper around [`crate::client::Client`].

use crate::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions,
};
use crate::models::{
    OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem, TranslationGroup,
    TranslationProject, ValidateApiKeyResponse,
};

use super::BlockingDriver;

/// Sync HTTP client. Do not call from an async Tokio task.
pub struct Client {
    inner: crate::client::Client,
    driver: BlockingDriver,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.inner.base_url())
            .finish_non_exhaustive()
    }
}

impl Client {
    pub(crate) fn from_parts(inner: crate::client::Client, driver: BlockingDriver) -> Self {
        Self { inner, driver }
    }

    pub(crate) fn into_parts(self) -> (crate::client::Client, BlockingDriver) {
        (self.inner, self.driver)
    }

    /// Wraps this client in a blocking [`super::Service`], reusing the same runtime.
    pub fn into_service(self) -> super::Service {
        let (inner, driver) = self.into_parts();
        super::Service::from_parts(crate::service::Service::new(inner), driver)
    }

    /// Default locale for convenience `Service` resolution.
    pub fn default_language(&self) -> Option<&str> {
        self.inner.default_language()
    }

    /// Default project id when set.
    pub fn default_project_id(&self) -> Option<&str> {
        self.inner.default_project_id()
    }

    /// Configured base URL.
    pub fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    /// Retrieves a single rendered translation string.
    pub fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        self.driver
            .block_on(self.inner.get_entry(group, entry, lang, opts))?
    }

    /// Retrieves one translation group for a project and language.
    pub fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        self.driver
            .block_on(self.inner.get_group(project, group, lang, opts))?
    }

    /// Retrieves all translation groups for a project and language.
    pub fn get_project(
        &self,
        project: &str,
        lang: &str,
        opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        self.driver
            .block_on(self.inner.get_project(project, lang, opts))?
    }

    /// Lists locales available for a project.
    pub fn get_project_locales(
        &self,
        project: &str,
        opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        self.driver
            .block_on(self.inner.get_project_locales(project, opts))?
    }

    /// Downloads the offline translation bundle as a ZIP archive.
    pub fn get_offline_cache(
        &self,
        project: &str,
        opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        self.driver
            .block_on(self.inner.get_offline_cache(project, opts))?
    }

    /// Reports missing translation keys (no-op when `keys` is empty).
    pub fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        self.driver.block_on(self.inner.report_missing_keys(keys))?
    }

    /// Validates the configured API key.
    pub fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        self.driver.block_on(self.inner.validate_api_key())?
    }
}
