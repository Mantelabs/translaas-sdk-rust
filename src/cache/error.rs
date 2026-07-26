//! Cache-specific errors.

use thiserror::Error;

/// Errors returned by cache providers and key helpers.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CacheError {
    /// Stored value type does not match the requested type on get.
    #[error("cache type mismatch")]
    TypeMismatch,
}
