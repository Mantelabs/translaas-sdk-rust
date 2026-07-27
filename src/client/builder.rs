//! Fluent builder for [`super::Client`].

use std::time::Duration;

#[cfg(feature = "cache")]
use std::sync::Arc;

use crate::models::read_json_ulid;
use crate::models::ConfigurationError;
use crate::validate::{self, ClientOptions as ValidateOptions};

#[cfg(feature = "cache")]
use crate::cache::{CacheMode, MemoryProvider, Ttl};

use super::options::{ClientOptions, DEFAULT_TIMEOUT};
use super::{Client, Error};

#[cfg(feature = "cache")]
use super::cache_integration::default_cache_provider;

/// Builds a validated [`Client`].
#[derive(Debug, Default)]
pub struct ClientBuilder {
    options: ClientOptions,
    http_client: Option<reqwest::Client>,
    #[cfg(feature = "cache")]
    cache_provider: Option<Arc<MemoryProvider>>,
}

impl ClientBuilder {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API key (`X-Api-Key`).
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.options.api_key = api_key.into();
        self
    }

    /// Sets the API base URL.
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.options.base_url = base_url.into();
        self
    }

    /// Sets the request timeout. Zero is treated as [`DEFAULT_TIMEOUT`].
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.options.timeout = Some(timeout);
        self
    }

    /// Sets the default project id used for text `project` query when unset on context.
    pub fn default_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.options.default_project_id = Some(project_id.into());
        self
    }

    /// Supplies a custom [`reqwest::Client`] (primarily for tests).
    pub fn http_client(mut self, http_client: reqwest::Client) -> Self {
        self.http_client = Some(http_client);
        self
    }

    /// Sets the in-memory cache mode (`None` disables caching).
    #[cfg(feature = "cache")]
    pub fn cache_mode(mut self, cache_mode: CacheMode) -> Self {
        self.options.cache_mode = cache_mode;
        self
    }

    /// Sets the TTL applied when storing cache entries.
    #[cfg(feature = "cache")]
    pub fn cache_ttl(mut self, cache_ttl: Ttl) -> Self {
        self.options.cache_ttl = cache_ttl;
        self
    }

    /// Supplies a custom in-memory cache provider (primarily for tests).
    #[cfg(feature = "cache")]
    pub fn cache_provider(mut self, cache_provider: Arc<MemoryProvider>) -> Self {
        self.cache_provider = Some(cache_provider);
        self
    }

    /// Validates options and builds the client.
    pub fn build(self) -> Result<Client, ConfigurationError> {
        let api_key = self.options.api_key.trim().to_string();
        let base_url = self.options.base_url.trim().to_string();
        let default_project_id = self
            .options
            .default_project_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let configured_timeout = self.options.timeout;
        validate::client(ValidateOptions {
            api_key: &api_key,
            base_url: &base_url,
            timeout: configured_timeout,
        })?;

        let timeout = match configured_timeout {
            Some(value) if !value.is_zero() => value,
            _ => DEFAULT_TIMEOUT,
        };

        let http_client = match self.http_client {
            Some(client) => client,
            None => reqwest::Client::builder()
                .use_rustls_tls()
                .timeout(timeout)
                .build()
                .map_err(|err| ConfigurationError {
                    message: format!("failed to build HTTP client: {err}"),
                })?,
        };

        Ok(Client {
            api_key,
            base_url,
            timeout,
            default_project_id,
            http_client,
            #[cfg(feature = "cache")]
            cache_mode: self.options.cache_mode,
            #[cfg(feature = "cache")]
            cache_ttl: self.options.cache_ttl,
            #[cfg(feature = "cache")]
            cache_provider: resolve_cache_provider(self.options.cache_mode, self.cache_provider),
        })
    }

    /// Builds a client, optionally calling [`Client::validate_api_key`] when
    /// `default_project_id` is unset and the key is scoped to a single project.
    pub async fn build_with_resolved_project(self) -> Result<Client, Error> {
        if self
            .options
            .default_project_id
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return self.build().map_err(Error::from);
        }

        let api_key = self.options.api_key.clone();
        let base_url = self.options.base_url.clone();
        let configured_timeout = self.options.timeout;
        let http_client = self.http_client.clone();
        #[cfg(feature = "cache")]
        let cache_mode = self.options.cache_mode;
        #[cfg(feature = "cache")]
        let cache_ttl = self.options.cache_ttl;
        #[cfg(feature = "cache")]
        let cache_provider = self.cache_provider.clone();

        let client = self.build().map_err(Error::from)?;
        let validate = client.validate_api_key().await?;
        let Some(project_id) = validate.project_id.as_ref().and_then(read_json_ulid) else {
            return Ok(client);
        };

        let mut builder = ClientBuilder::new()
            .api_key(api_key)
            .base_url(base_url)
            .default_project_id(project_id);
        if let Some(timeout) = configured_timeout {
            builder = builder.timeout(timeout);
        }
        if let Some(http_client) = http_client {
            builder = builder.http_client(http_client);
        }
        #[cfg(feature = "cache")]
        {
            builder = builder.cache_mode(cache_mode).cache_ttl(cache_ttl);
            if let Some(cache_provider) = cache_provider {
                builder = builder.cache_provider(cache_provider);
            }
        }
        builder.build().map_err(Error::from)
    }
}

#[cfg(feature = "cache")]
fn resolve_cache_provider(
    cache_mode: CacheMode,
    override_provider: Option<Arc<MemoryProvider>>,
) -> Option<Arc<MemoryProvider>> {
    if cache_mode == CacheMode::None {
        return None;
    }
    Some(
        override_provider
            .unwrap_or_else(|| default_cache_provider(cache_mode).expect("non-none mode")),
    )
}
