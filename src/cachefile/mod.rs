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
//! Hybrid L1/L2 composition, decorator clients, and background sync arrive in
//! later issues (#9–#11).

#![warn(missing_docs)]

mod atomic;
mod file_provider;
mod paths;
mod provider;
mod types;

pub use file_provider::{FileProvider, FileProviderOptions};
pub use paths::sanitize_path_segment;
pub use provider::{Provider, SaveOptions};
pub use types::{
    CacheManifest, CachedLocales, CachedProject, ProjectCacheInfo, DEFAULT_SDK_VERSION,
    MANIFEST_VERSION,
};
