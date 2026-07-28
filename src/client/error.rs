//! Public error type for the HTTP client.

use thiserror::Error;

use crate::models::{ApiError, ConfigurationError, OfflineCacheError, OfflineCacheMissError};

/// Errors returned by [`super::Client`] operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid client options or call arguments.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    /// Non-success HTTP response or mapped transport failure.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// Offline cache I/O or deserialization failure.
    #[error(transparent)]
    OfflineCache(Box<OfflineCacheError>),
    /// Expected data was not found in the offline cache.
    #[error(transparent)]
    OfflineCacheMiss(Box<OfflineCacheMissError>),
    /// The request was canceled before completion.
    #[error("request was canceled")]
    Canceled,
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
