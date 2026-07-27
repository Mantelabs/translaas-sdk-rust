//! JSON wrapper and manifest types for the on-disk offline cache.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{ProjectLocales, TranslationProject};

/// Root manifest schema version written by the SDK.
pub const MANIFEST_VERSION: &str = "1.0";

/// Offline cache format version recorded in `manifest.json`.
pub const DEFAULT_SDK_VERSION: &str = "1.0.0";

/// Wraps a translation project payload with cache metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedProject {
    /// UTC timestamp when the entry was written.
    pub cached_at: DateTime<Utc>,
    /// Optional absolute expiry; omitted when the entry does not expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Cached project payload.
    pub data: TranslationProject,
}

/// Wraps supported locales with cache metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedLocales {
    /// UTC timestamp when the entry was written.
    pub cached_at: DateTime<Utc>,
    /// Optional absolute expiry; omitted when the entry does not expire.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// Cached locales payload.
    pub data: ProjectLocales,
}

/// Root offline cache index (`manifest.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheManifest {
    /// Manifest schema version.
    pub version: String,
    /// SDK offline format version.
    pub sdk_version: String,
    /// UTC timestamp when the cache was first created.
    pub created_at: DateTime<Utc>,
    /// UTC timestamp of the most recent manifest update.
    pub last_sync_at: DateTime<Utc>,
    /// Per-project cache metadata keyed by sanitized project id.
    pub projects: HashMap<String, ProjectCacheInfo>,
}

/// Tracks cached languages for one project.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCacheInfo {
    /// Language codes with cached project payloads.
    pub languages: Vec<String>,
    /// UTC timestamp of the last sync for this project.
    pub last_sync_at: DateTime<Utc>,
    /// Sync status label (for example `"synced"`).
    pub status: String,
}

impl Default for CacheManifest {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            version: MANIFEST_VERSION.to_string(),
            sdk_version: DEFAULT_SDK_VERSION.to_string(),
            created_at: now,
            last_sync_at: now,
            projects: HashMap::new(),
        }
    }
}
