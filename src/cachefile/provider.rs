//! Offline disk cache provider trait and save options.

use chrono::{DateTime, Utc};

use crate::models::{OfflineCacheError, ProjectLocales, TranslationGroup, TranslationProject};

use super::types::CacheManifest;

/// Options applied when persisting wrapped cache entries.
#[derive(Debug, Clone)]
pub struct SaveOptions {
    /// UTC timestamp recorded as `cachedAt` on the wrapper.
    pub cached_at: DateTime<Utc>,
    /// Optional wrapper expiry (`expiresAt`).
    pub expires_at: Option<DateTime<Utc>>,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            cached_at: Utc::now(),
            expires_at: None,
        }
    }
}

impl SaveOptions {
    /// Creates save options with the current UTC time as `cachedAt`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets wrapper `expiresAt` (`None` clears expiry).
    pub fn with_expires_at(mut self, expires_at: Option<DateTime<Utc>>) -> Self {
        self.expires_at = expires_at;
        self
    }
}

/// Offline disk cache contract (L2). Distinct from [`crate::cache::Provider`].
pub trait Provider: Send + Sync {
    /// Returns cached project data, or `None` on miss or expiry.
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError>;

    /// Writes project data to disk and updates the root manifest.
    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError>;

    /// Returns a group extracted from the cached project payload.
    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError>;

    /// Returns cached locales, falling back to manifest or locale directories.
    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError>;

    /// Writes `locales.json` and updates the root manifest.
    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError>;

    /// Reads root `manifest.json`, or `None` when absent.
    fn get_manifest(&self) -> Result<Option<CacheManifest>, OfflineCacheError>;

    /// Read-modify-writes `manifest.json` atomically.
    fn update_manifest(
        &self,
        update: &mut dyn FnMut(&mut CacheManifest) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError>;

    /// Reports whether a non-expired project/language payload exists on disk.
    fn is_cached(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError>;

    /// Removes the entire cache directory tree.
    fn clear(&self) -> Result<(), OfflineCacheError>;
}

impl<P: Provider + ?Sized> Provider for std::sync::Arc<P> {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        self.as_ref().get_project(project, lang)
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        self.as_ref().save_project(project, lang, data, options)
    }

    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        self.as_ref().get_group(project, group, lang)
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        self.as_ref().get_locales(project)
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        self.as_ref().save_locales(project, data, options)
    }

    fn get_manifest(&self) -> Result<Option<CacheManifest>, OfflineCacheError> {
        self.as_ref().get_manifest()
    }

    fn update_manifest(
        &self,
        update: &mut dyn FnMut(&mut CacheManifest) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        self.as_ref().update_manifest(update)
    }

    fn is_cached(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError> {
        self.as_ref().is_cached(project, lang)
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        self.as_ref().clear()
    }
}
