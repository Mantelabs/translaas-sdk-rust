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
//! Decorator clients and background sync arrive in later issues (#10–#11).

#![warn(missing_docs)]

mod atomic;
mod file_provider;
mod hybrid_options;
mod hybrid_provider;
mod paths;
mod provider;
mod types;

pub use file_provider::{FileProvider, FileProviderOptions};
pub use hybrid_options::HybridOptions;
pub use hybrid_provider::HybridProvider;
pub use paths::sanitize_path_segment;
pub use provider::{Provider, SaveOptions};
pub use types::{
    CacheManifest, CachedLocales, CachedProject, ProjectCacheInfo, DEFAULT_SDK_VERSION,
    MANIFEST_VERSION,
};
