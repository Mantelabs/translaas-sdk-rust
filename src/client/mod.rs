//! HTTP client, options validation, and live translation reads.

mod builder;
mod error;
mod get_entry;
mod get_group;
mod get_offline_cache;
mod get_project;
mod get_project_locales;
mod json_get;
mod options;
mod report_missing_keys;
mod r#trait;
mod transport;
mod validate_api_key;

use std::time::Duration;

pub use builder::ClientBuilder;
pub use error::Error;
pub use get_entry::GetEntryOptions;
pub use get_group::GetGroupOptions;
pub use get_offline_cache::GetOfflineCacheOptions;
pub use get_project::GetProjectOptions;
pub use get_project_locales::GetProjectLocalesOptions;
pub use options::{ClientOptions, DEFAULT_TIMEOUT};
pub use r#trait::TranslaasClient;

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
