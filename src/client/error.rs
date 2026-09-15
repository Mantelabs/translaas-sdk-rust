//! Public error type for the HTTP client.

use thiserror::Error;

use crate::models::{ApiError, ConfigurationError, OfflineCacheError, OfflineCacheMissError};

/// Errors returned by [`super::Client`] operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid client options or call arguments.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    /// Non-success HTTP response or timeout mapped to status 408.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Connect, TLS, DNS, or other send failure (not an HTTP status from the API).
    #[error("transport error: {message}")]
    Transport { message: String },
    /// Offline cache I/O or deserialization failure.
    #[error(transparent)]
    OfflineCache(Box<OfflineCacheError>),
    /// Expected data was not found in the offline cache.
    #[error(transparent)]
    OfflineCacheMiss(Box<OfflineCacheMissError>),
    /// The request was canceled before completion.
    #[error("request was canceled")]
    Canceled,
    /// A blocking API was invoked from an async runtime (would deadlock).
    #[error(
        "blocking Translaas API cannot be used from an async runtime; use async Client/Service or call from a non-async thread"
    )]
    BlockingInAsyncContext,
}

impl Error {
    /// Returns the API error when this is an [`Error::Api`] variant.
    pub fn as_api(&self) -> Option<&ApiError> {
        match self {
            Self::Api(err) => Some(err),
            _ => None,
        }
    }

    /// Returns true when the error is a user/request cancellation.
    pub fn is_canceled(&self) -> bool {
        matches!(self, Self::Canceled)
    }

    /// Returns true when the error is a connect/TLS/DNS (or similar) send failure.
    pub fn is_transport(&self) -> bool {
        matches!(self, Self::Transport { .. })
    }

    /// Returns the offline cache miss error when this is an [`Error::OfflineCacheMiss`] variant.
    pub fn as_offline_cache_miss(&self) -> Option<&OfflineCacheMissError> {
        match self {
            Self::OfflineCacheMiss(err) => Some(err.as_ref()),
            _ => None,
        }
    }

    /// Returns true when the error is an offline cache miss.
    pub fn is_offline_cache_miss(&self) -> bool {
        matches!(self, Self::OfflineCacheMiss(_))
    }

    /// Returns true when a blocking API was called from an async runtime.
    pub fn is_blocking_in_async_context(&self) -> bool {
        matches!(self, Self::BlockingInAsyncContext)
    }
}

impl From<OfflineCacheError> for Error {
    fn from(value: OfflineCacheError) -> Self {
        Self::OfflineCache(Box::new(value))
    }
}

impl From<OfflineCacheMissError> for Error {
    fn from(value: OfflineCacheMissError) -> Self {
        Self::OfflineCacheMiss(Box::new(value))
    }
}
