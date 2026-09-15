//! Sync [`ClientBuilder`](crate::client::ClientBuilder) analogue.

use crate::client::ClientBuilder as AsyncBuilder;
use crate::models::ConfigurationError;

#[cfg(feature = "cache")]
use std::sync::Arc;

#[cfg(feature = "cache")]
use crate::cache::{CacheMode, MemoryProvider, Ttl};

use super::{BlockingDriver, Client};

/// Builds a blocking [`Client`] with the same options as the async builder.
#[derive(Debug, Default)]
pub struct ClientBuilder {
    inner: AsyncBuilder,
}

impl ClientBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API key (`X-Api-Key`).
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.inner = self.inner.api_key(api_key);
        self
    }

    /// Sets the API base URL.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self.inner.base_url(base_url);
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// Sets the default project id used for text `project` query when unset on context.
    pub fn default_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.inner = self.inner.default_project_id(project_id);
        self
    }

    /// Sets the default locale used by the convenience Service when no explicit lang is given.
    pub fn default_language(mut self, language: impl Into<String>) -> Self {
        self.inner = self.inner.default_language(language);
        self
    }

    /// Accept invalid TLS certificates on the internal HTTP client (**dev-only**).
    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.inner = self.inner.accept_invalid_certs(accept);
        self
    }

    /// Supplies a custom [`reqwest::Client`] (primarily for tests).
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.inner = self.inner.http_client(http_client);
        self
    }

    /// Sets the in-memory cache mode (`None` disables caching).
    #[cfg(feature = "cache")]
    pub fn cache_mode(mut self, cache_mode: CacheMode) -> Self {
        self.inner = self.inner.cache_mode(cache_mode);
        self
    }

    /// Sets the TTL applied when storing cache entries.
    #[cfg(feature = "cache")]
    pub fn cache_ttl(mut self, cache_ttl: Ttl) -> Self {
        self.inner = self.inner.cache_ttl(cache_ttl);
        self
    }

    /// Supplies a custom in-memory cache provider (primarily for tests).
    #[cfg(feature = "cache")]
    pub fn cache_provider(mut self, cache_provider: Arc<MemoryProvider>) -> Self {
        self.inner = self.inner.cache_provider(cache_provider);
        self
    }

    /// Validates options and builds a blocking client.
    pub fn build(self) -> Result<Client, ConfigurationError> {
        let inner = self.inner.build()?;
        let driver = BlockingDriver::new()?;
        Ok(Client::from_parts(inner, driver))
    }
}
