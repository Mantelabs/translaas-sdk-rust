//! In-memory cache provider trait.

use super::{CacheError, Ttl};

/// In-memory cache boundary consumed by the HTTP client (wired in issue #7).
///
/// Values must be [`Clone`] so callers receive owned copies on hit. The client
/// will cache concrete model types such as `String`, `TranslationGroup`, and
/// `TranslationProject`.
pub trait Provider: Send + Sync {
    /// Returns a cloned value on hit, [`None`] on miss, or an error on type mismatch.
    fn get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Result<Option<T>, CacheError>;

    /// Stores a value under `key` with optional expiration.
    fn set<T: Clone + Send + Sync + 'static>(
        &self,
        key: &str,
        value: T,
        ttl: Ttl,
    ) -> Result<(), CacheError>;

    /// Removes a single entry. Missing keys are not an error.
    fn remove(&self, key: &str) -> Result<(), CacheError>;

    /// Removes all entries.
    fn clear(&self) -> Result<(), CacheError>;
}
