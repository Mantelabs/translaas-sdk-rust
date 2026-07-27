//! Client construction options and defaults.

use std::time::Duration;

#[cfg(feature = "cache")]
use crate::cache::{CacheMode, Ttl};

/// Default HTTP timeout when none is configured (`30s`, matching Go/ .NET).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Configuration for [`super::Client`] construction.
///
/// Prefer [`super::ClientBuilder`] for a fluent API.
#[derive(Debug, Clone, Default)]
pub struct ClientOptions {
    /// API key sent as `X-Api-Key`.
    pub api_key: String,
    /// Base URL (origin or origin + path prefix before `/sdk/...`).
    pub base_url: String,
    /// Request timeout. `None` or zero → [`DEFAULT_TIMEOUT`].
    pub timeout: Option<Duration>,
    /// Default `project` query value for text lookups when request context omits it.
    pub default_project_id: Option<String>,
    /// In-memory cache mode. Default [`CacheMode::None`] disables caching.
    #[cfg(feature = "cache")]
    pub cache_mode: CacheMode,
    /// TTL applied when storing cache entries.
    #[cfg(feature = "cache")]
    pub cache_ttl: Ttl,
}
