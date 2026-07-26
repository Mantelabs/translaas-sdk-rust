//! HTTP client, options validation, and live translation reads.
//!
//! Phase 1 exposes [`Client::get_entry`] for `GET /sdk/v1/translations/text`.
//! Caching and additional endpoints arrive in later issues.

mod builder;
mod error;
mod get_entry;
mod options;
mod transport;

use std::time::Duration;

pub use builder::ClientBuilder;
pub use error::Error;
pub use get_entry::GetEntryOptions;
pub use options::{ClientOptions, DEFAULT_TIMEOUT};

/// Live Translaas HTTP client (no caching).
#[derive(Debug, Clone)]
pub struct Client {
    pub(crate) api_key: String,
    pub(crate) base_url: String,
    pub(crate) timeout: Duration,
    pub(crate) default_project_id: Option<String>,
    pub(crate) http_client: reqwest::Client,
}

impl Client {
    /// Starts a new [`ClientBuilder`].
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Configured request timeout.
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Configured base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Default project id when set.
    pub fn default_project_id(&self) -> Option<&str> {
        self.default_project_id.as_deref()
    }
}
