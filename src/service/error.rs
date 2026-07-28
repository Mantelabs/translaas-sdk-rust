//! Errors returned by the convenience translation API.

use thiserror::Error;

use crate::client;
use crate::models::NoLanguageError;

/// Errors from [`super::Service::t`](super::Service::t) and related helpers.
#[derive(Debug, Error)]
pub enum Error {
    /// Language resolution yielded no language.
    #[error(transparent)]
    NoLanguage(#[from] NoLanguageError),

    /// The underlying HTTP client failed.
    #[error(transparent)]
    Client(#[from] client::Error),

    /// Language resolver configuration failed.
    #[error(transparent)]
    Language(#[from] super::language::LanguageResolverError),
}

impl Error {
    /// Returns `true` when language resolution failed before calling the client.
    pub fn is_no_language(&self) -> bool {
        matches!(self, Self::NoLanguage(_))
    }
}
