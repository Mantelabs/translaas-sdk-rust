//! Umbrella offline configuration aligned with Go / .NET §4.3.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::models::ConfigurationError;

use super::caching_options::{CachingOptions, FallbackMode};

const DEFAULT_CACHE_DIRECTORY: &str = ".translaas-cache";
const DEFAULT_AUTO_SYNC_INTERVAL_SECS: u64 = 3600;

/// Full offline stack configuration (decorator, sync, and file provider).
///
/// Use [`Self::caching_options`] to derive narrow [`CachingOptions`] for
/// [`super::CachingClient`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineCacheOptions {
    /// Master switch for app/DI layers. Sync methods do not no-op when false.
    pub enabled: bool,
    /// Root cache directory for [`super::FileProvider`].
    pub cache_directory: PathBuf,
    /// Cache vs API ordering for [`super::CachingClient`] reads.
    pub fallback_mode: FallbackMode,
    /// Enables periodic background sync when [`super::SyncService::start_background_sync`] is used.
    pub auto_sync: bool,
    /// Interval between background sync runs. `None` disables interval-based sync.
    pub auto_sync_interval: Option<Duration>,
    /// Project IDs synced by [`super::SyncService::sync_all`] and background sync.
    pub projects: Vec<String>,
    /// Limits sync to these locale codes. Empty means all project locales from the API.
    pub languages: Vec<String>,
    /// Default project for entry-level offline lookups via [`super::CachingClient`].
    pub default_project_id: String,
}

impl Default for OfflineCacheOptions {
    fn default() -> Self {
        Self::default_offline_cache_options()
    }
}

impl OfflineCacheOptions {
    /// Returns options aligned with Go `DefaultOfflineCacheOptions` / .NET defaults.
    pub fn default_offline_cache_options() -> Self {
        Self {
            enabled: false,
            cache_directory: PathBuf::from(DEFAULT_CACHE_DIRECTORY),
            fallback_mode: FallbackMode::CacheFirst,
            auto_sync: true,
            auto_sync_interval: Some(Duration::from_secs(DEFAULT_AUTO_SYNC_INTERVAL_SECS)),
            projects: Vec::new(),
            languages: Vec::new(),
            default_project_id: String::new(),
        }
    }

    /// Sets the cache root directory.
    pub fn with_cache_directory(mut self, path: impl AsRef<Path>) -> Self {
        self.cache_directory = path.as_ref().to_path_buf();
        self
    }

    /// Maps umbrella options to [`CachingOptions`] for [`super::CachingClient::new`].
    pub fn caching_options(&self) -> Result<CachingOptions, ConfigurationError> {
        if self.default_project_id.trim().is_empty() {
            return Err(ConfigurationError {
                message: "cachefile: default_project_id must not be empty".to_string(),
            });
        }
        Ok(CachingOptions {
            fallback_mode: self.fallback_mode,
            default_project_id: self.default_project_id.trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_offline_cache_options_match_go() {
        let opts = OfflineCacheOptions::default_offline_cache_options();
        assert_eq!(opts.cache_directory, PathBuf::from(".translaas-cache"));
        assert_eq!(opts.fallback_mode, FallbackMode::CacheFirst);
        assert!(opts.auto_sync);
        assert_eq!(opts.auto_sync_interval, Some(Duration::from_secs(3600)));
        assert!(opts.projects.is_empty());
        assert!(opts.languages.is_empty());
        assert!(opts.default_project_id.is_empty());
    }

    #[test]
    fn caching_options_rejects_empty_default_project_id() {
        let opts = OfflineCacheOptions::default_offline_cache_options();
        assert!(opts.caching_options().is_err());
    }

    #[test]
    fn caching_options_maps_fields() {
        let mut opts = OfflineCacheOptions::default_offline_cache_options();
        opts.default_project_id = "demo".to_string();
        opts.fallback_mode = FallbackMode::ApiFirst;

        let caching = opts.caching_options().expect("valid options");
        assert_eq!(caching.default_project_id, "demo");
        assert_eq!(caching.fallback_mode, FallbackMode::ApiFirst);
    }
}
