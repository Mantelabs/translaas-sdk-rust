//! Fluent builder for [`super::Client`].

use std::time::Duration;

use crate::models::ConfigurationError;
use crate::validate::{self, ClientOptions as ValidateOptions};

use super::options::{ClientOptions, DEFAULT_TIMEOUT};
use super::Client;

/// Builds a validated [`Client`].
#[derive(Debug, Default)]
pub struct ClientBuilder {
    options: ClientOptions,
    http_client: Option<reqwest::Client>,
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
        })
    }
}
