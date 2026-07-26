//! Public error type for the HTTP client.

use thiserror::Error;

use crate::models::{ApiError, ConfigurationError};

/// Errors returned by [`super::Client`] operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid client options or call arguments.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    /// Non-success HTTP response or mapped transport failure.
    #[error(transparent)]
    Api(#[from] ApiError),
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
}
