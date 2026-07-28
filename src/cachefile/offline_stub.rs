//! Inner client stub for keyless [`CacheOnly`](super::FallbackMode::CacheOnly) usage.

use crate::client::{
    Error, GetEntryOptions, GetGroupOptions, GetOfflineCacheOptions, GetProjectLocalesOptions,
    GetProjectOptions, TranslaasClient,
};
use crate::models::{
    ConfigurationError, OfflineCacheDownloadResult, ProjectLocales, ReportMissingKeyItem,
    TranslationGroup, TranslationProject, ValidateApiKeyResponse,
};

/// Placeholder inner client for offline-only deployments without an API key.
///
/// Intercepted read methods are never invoked when [`FallbackMode::CacheOnly`](super::FallbackMode::CacheOnly)
/// is configured. Passthrough methods return a configuration error if called.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OfflineStubClient;

impl OfflineStubClient {
    /// Creates a new offline stub inner client.
    pub fn new() -> Self {
        Self
    }
}

fn offline_only_configuration_error(method: &str) -> Error {
    Error::Configuration(ConfigurationError {
        message: format!(
            "OfflineStubClient does not support {method}; configure a live client for passthrough operations"
        ),
    })
}

impl TranslaasClient for OfflineStubClient {
    async fn get_entry(
        &self,
        _group: &str,
        _entry: &str,
        _lang: &str,
        _opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        Err(offline_only_configuration_error("get_entry"))
    }

    async fn get_group(
        &self,
        _project: &str,
        _group: &str,
        _lang: &str,
        _opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        Err(offline_only_configuration_error("get_group"))
    }

    async fn get_project(
        &self,
        _project: &str,
        _lang: &str,
        _opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        Err(offline_only_configuration_error("get_project"))
    }

    async fn get_project_locales(
        &self,
        _project: &str,
        _opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        Err(offline_only_configuration_error("get_project_locales"))
    }

    async fn get_offline_cache(
        &self,
        _project: &str,
        _opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        Err(offline_only_configuration_error("get_offline_cache"))
    }

    async fn report_missing_keys(&self, _keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        Err(offline_only_configuration_error("report_missing_keys"))
    }

    async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        Err(offline_only_configuration_error("validate_api_key"))
    }
}
