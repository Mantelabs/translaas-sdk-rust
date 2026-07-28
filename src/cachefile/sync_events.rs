//! Sync lifecycle events and optional callbacks.

use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::client::Error;

/// Reports a successful project/language sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCompletedEvent {
    /// Project identifier.
    pub project: String,
    /// Language code.
    pub language: String,
    /// UTC timestamp when the sync completed.
    pub synced_at: DateTime<Utc>,
}

/// Reports a failed project/language sync.
#[derive(Debug)]
pub struct SyncFailedEvent {
    /// Project identifier.
    pub project: String,
    /// Language code.
    pub language: String,
    /// Underlying error from the API or cache layer.
    pub error: Error,
}

/// Aggregates [`super::SyncService::sync_all`] outcomes across configured projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncResult {
    /// Projects that synced successfully.
    pub synced_projects: Vec<String>,
    /// Projects that failed during sync-all.
    pub failed_projects: Vec<String>,
    /// UTC timestamp when sync-all finished.
    pub completed_at: DateTime<Utc>,
}

/// Unified sync event for channel adapters.
#[derive(Debug)]
pub enum SyncEvent {
    /// A single project/language sync completed.
    Completed(SyncCompletedEvent),
    /// A single project/language sync failed.
    Failed(SyncFailedEvent),
    /// Sync-all finished.
    AllCompleted(SyncResult),
}

/// Optional hooks for sync lifecycle events (Go `SyncCallbacks` parity).
#[derive(Clone, Default)]
pub struct SyncCallbacks {
    /// Called after a successful project/language sync.
    pub on_sync_completed: Option<Arc<dyn Fn(SyncCompletedEvent) + Send + Sync>>,
    /// Called after a failed project/language sync.
    pub on_sync_failed: Option<Arc<dyn Fn(SyncFailedEvent) + Send + Sync>>,
    /// Called after sync-all completes.
    pub on_sync_all_completed: Option<Arc<dyn Fn(SyncResult) + Send + Sync>>,
}
