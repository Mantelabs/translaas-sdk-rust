//! Options for the offline [`CachingClient`](super::CachingClient) decorator.

use std::fmt;

/// Controls cache vs API ordering for intercepted read operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum FallbackMode {
    /// Reads disk first, then API on miss.
    #[default]
    CacheFirst,
    /// Reads API first, then disk on compatible network/API failures.
    ApiFirst,
    /// Reads disk only; never calls the inner client for intercepted reads.
    CacheOnly,
}

impl fmt::Display for FallbackMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CacheFirst => "CacheFirst",
            Self::ApiFirst => "ApiFirst",
            Self::CacheOnly => "CacheOnly",
        })
    }
}

/// Configures offline fallback behavior for [`CachingClient`](super::CachingClient).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachingOptions {
    /// Cache vs API ordering for read methods.
    pub fallback_mode: FallbackMode,
    /// Default project id used for entry-level offline lookups.
    pub default_project_id: String,
}

impl Default for CachingOptions {
    fn default() -> Self {
        Self {
            fallback_mode: FallbackMode::CacheFirst,
            default_project_id: String::new(),
        }
    }
}

impl CachingOptions {
    /// Returns options aligned with .NET / Go defaults (`CacheFirst`).
    pub fn cache_first() -> Self {
        Self::default()
    }
}
