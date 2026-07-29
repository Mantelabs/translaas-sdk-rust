//! On-disk offline cache (L2) for the Translaas SDK.
//!
//! [`FileProvider`] persists translation payloads under a root directory using
//! JSON wrappers, a root [`CacheManifest`], path sanitization, and atomic
//! `*.tmp` writes aligned with the Go and .NET SDKs.
//!
//! # Layout
//!
//! ```text
//! {CacheDirectory}/
//! ├── manifest.json
//! └── {sanitizedProjectId}/
//!     ├── locales.json
//!     └── {sanitizedLang}/
//!         └── project.json
//! ```
//!
//! # Blocking I/O
//!
//! Disk operations use synchronous [`std::fs`] calls. When calling from async
//! code, prefer [`tokio::task::spawn_blocking`] (or equivalent) to avoid blocking
//! the runtime.
//!
//! # Hybrid L1-over-L2
//!
//! [`HybridProvider`] adds an expirable LRU memory layer (L1) over any L2
//! [`Provider`] (typically [`FileProvider`]). Defaults: enabled, 30 minute TTL,
//! 1000 entries per partition. L1 uses the [`lru`](https://docs.rs/lru) crate with
//! explicit TTL (see [`HybridProvider`] for notes on `moka` / `quick_cache`).
//!
//! This is distinct from HTTP in-memory caching in [`crate::cache::MemoryProvider`].
//!
//! # Offline decorator
//!
//! [`CachingClient`] wraps [`crate::client::TranslaasClient`] with
//! [`FallbackMode`] strategies for read operations.
//!
//! # Sync service
//!
//! [`SyncService`] pulls translations from the API into a [`Provider`] using the
//! **inner** client (never [`CachingClient`]). Optional background sync uses a
//! Tokio interval and [`tokio_util::sync::CancellationToken`].
//!
//! Offline ZIP bundles (HTTP spec §7.6) can be imported with
//! [`FileProvider::import_offline_bundle`] or downloaded and imported via
//! [`SyncService::sync_from_offline_zip`].

#![warn(missing_docs)]

mod atomic;
mod caching_client;
mod caching_options;
mod fallback;
mod file_provider;
mod file_provider_import;
mod hybrid_options;
mod hybrid_provider;
mod offline_cache_options;
mod offline_entry;
mod offline_stub;
mod paths;
mod provider;
mod sync_events;
mod sync_language_filter;
mod sync_service;
mod types;
mod update_group_cache;
mod zip_bundle;

pub use caching_client::CachingClient;
pub use caching_options::{CachingOptions, FallbackMode};
pub use file_provider::{FileProvider, FileProviderOptions};
pub use hybrid_options::HybridOptions;
pub use hybrid_provider::HybridProvider;
pub use offline_cache_options::OfflineCacheOptions;
pub use offline_stub::OfflineStubClient;
pub use paths::sanitize_path_segment;
pub use provider::{Provider, SaveOptions};
pub use sync_events::{SyncCallbacks, SyncCompletedEvent, SyncEvent, SyncFailedEvent, SyncResult};
pub use sync_service::SyncService;
pub use types::{
    CacheManifest, CachedLocales, CachedProject, ProjectCacheInfo, DEFAULT_SDK_VERSION,
    MANIFEST_VERSION,
};
pub use zip_bundle::{parse_offline_zip, resolve_project_key, OfflineBundle};
