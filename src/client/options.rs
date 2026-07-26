//! Client construction options and defaults.

use std::time::Duration;

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
}
