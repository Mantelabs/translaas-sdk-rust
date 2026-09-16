//! Axum installer analogue of .NET `AddTranslaas` (builder + layer, not a DI container).

use std::sync::Arc;
use std::time::Duration;

use axum::middleware::from_fn_with_state;
use axum::Router;
use thiserror::Error;

#[cfg(feature = "cache")]
use crate::cache::CacheMode;
use crate::client::ClientBuilder;
use crate::models::ConfigurationError;

use super::error::MiddlewareError;
use super::middleware::{middleware, translaas_middleware, MiddlewareOptions};

const DEFAULT_BASE_URL: &str = "https://api.translaas.local";
const DEFAULT_PROJECT: &str = "translaassdksamples";
const DEFAULT_LANGUAGE: &str = "en";

/// Fluent installer configuration (getting-started parity with [`crate::ClientBuilder`]).
///
/// Custom HTTP clients, language sources, or offline `CachingClient` stay on the
/// manual [`middleware`](super::middleware::middleware) path.
#[derive(Debug, Clone, Default)]
pub struct InstallOptions {
    api_key: String,
    base_url: String,
    default_project_id: Option<String>,
    default_language: Option<String>,
    accept_invalid_certs: bool,
    timeout: Option<Duration>,
    #[cfg(feature = "cache")]
    cache_mode: Option<CacheMode>,
}

impl InstallOptions {
    /// Creates empty options. [`Self::api_key`] and [`Self::base_url`] must be set before install.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the API key (`X-Api-Key`).
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// Sets the API base URL (origin only; do not append `/sdk`).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Sets the default project slug (or ULID) for text lookups.
    pub fn default_project_id(mut self, project_id: impl Into<String>) -> Self {
        self.default_project_id = Some(project_id.into());
        self
    }

    /// Sets the default locale used when the request does not resolve a language.
    pub fn default_language(mut self, language: impl Into<String>) -> Self {
        self.default_language = Some(language.into());
        self
    }

    /// Accept invalid TLS certificates on the internal HTTP client.
    ///
    /// **Dev-only** — for local Docker (`https://api.translaas.local` with self-signed certs).
    /// Do not enable in production.
    pub fn accept_invalid_certs(mut self, accept: bool) -> Self {
        self.accept_invalid_certs = accept;
        self
    }

    /// Sets the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the in-memory cache mode.
    #[cfg(feature = "cache")]
    pub fn cache_mode(mut self, cache_mode: CacheMode) -> Self {
        self.cache_mode = Some(cache_mode);
        self
    }

    /// Builds options from process environment variables.
    ///
    /// | Variable | Required | Default |
    /// |----------|----------|---------|
    /// | `TRANSLAAS_API_KEY` | yes | — |
    /// | `TRANSLAAS_BASE_URL` | no | `https://api.translaas.local` |
    /// | `TRANSLAAS_DEFAULT_PROJECT` | no | `translaassdksamples` |
    /// | `TRANSLAAS_DEFAULT_LANGUAGE` | no | `en` |
    /// | `TRANSLAAS_TLS_INSECURE` | no | `1` / `true` / `TRUE` enables invalid TLS |
    /// | `TRANSLAAS_CACHE_MODE` | no | `none` / `entry` / `group` / `project` |
    ///
    /// `accept_invalid_certs(true)` is also set when the base URL contains `.translaas.local`.
    pub fn from_env() -> Result<Self, InstallError> {
        options_from_map(|key| std::env::var(key).ok())
    }

    fn into_builder(self) -> Result<ClientBuilder, InstallError> {
        if self.api_key.trim().is_empty() {
            return Err(InstallError::MissingApiKey);
        }

        let mut builder = ClientBuilder::new()
            .api_key(self.api_key)
            .base_url(self.base_url)
            .accept_invalid_certs(self.accept_invalid_certs);

        if let Some(project) = self.default_project_id {
            builder = builder.default_project_id(project);
        }
        if let Some(language) = self.default_language {
            builder = builder.default_language(language);
        }
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        #[cfg(feature = "cache")]
        if let Some(mode) = self.cache_mode {
            builder = builder.cache_mode(mode);
        }

        Ok(builder)
    }
}

/// Install-time failures (configuration, env, middleware setup).
#[derive(Debug, Error)]
pub enum InstallError {
    /// API key was missing or whitespace-only.
    #[error("ApiKey is required and cannot be null or empty.")]
    MissingApiKey,
    /// [`ClientBuilder`](crate::client::ClientBuilder) rejected the options.
    #[error(transparent)]
    Configuration(#[from] ConfigurationError),
    /// Middleware construction failed (should not happen on the installer happy path).
    #[error(transparent)]
    Middleware(#[from] MiddlewareError),
    /// Environment variable missing or invalid.
    #[error("{variable}: {message}")]
    Env {
        /// Variable name.
        variable: &'static str,
        /// Human-readable reason.
        message: String,
    },
}

impl InstallError {
    /// Returns `true` when the API key was missing or empty.
    pub fn is_missing_api_key(&self) -> bool {
        matches!(self, Self::MissingApiKey)
    }
}

impl From<std::env::VarError> for InstallError {
    fn from(_: std::env::VarError) -> Self {
        Self::MissingApiKey
    }
}

/// Builds a live [`crate::client::Client`] + [`crate::service::Service`], installs request-language
/// middleware, and returns `router` with the layer attached.
///
/// Does **not** call [`Router::with_state`] with Translaas middleware state. The app still owns `S`.
/// Handlers extract [`super::Translaas<crate::client::Client>`] from request extensions.
///
/// ```no_run
/// use axum::{routing::get, Router};
/// use translaas::axum::{add_translaas, Translaas};
/// use translaas::client::Client;
///
/// # fn demo() -> Result<(), translaas::axum::InstallError> {
/// let app = add_translaas(Router::new().route("/", get(welcome)), |o| {
///     Ok(o.api_key(std::env::var("TRANSLAAS_API_KEY")?)
///         .base_url("https://api.translaas.local")
///         .default_project_id("translaassdksamples")
///         .default_language("en")
///         .accept_invalid_certs(true))
/// })?;
/// # let _ = app;
/// # Ok(())
/// # }
///
/// async fn welcome(
///     Translaas(t): Translaas<Client>,
/// ) -> Result<String, (axum::http::StatusCode, String)> {
///     t.t("common", "welcome.message")
///         .await
///         .map_err(|e| (axum::http::StatusCode::BAD_GATEWAY, e.to_string()))
/// }
/// ```
pub fn add_translaas<S>(
    router: Router<S>,
    configure: impl FnOnce(InstallOptions) -> Result<InstallOptions, InstallError>,
) -> Result<Router<S>, InstallError>
where
    S: Clone + Send + Sync + 'static,
{
    let opts = configure(InstallOptions::new())?;
    let service = opts
        .into_builder()?
        .build_service()
        .map_err(install_error_from_service)?;
    let state = Arc::new(middleware(MiddlewareOptions::with_base_service(service))?);
    Ok(router.layer(from_fn_with_state(state, translaas_middleware)))
}

/// [`add_translaas`] using [`InstallOptions::from_env`].
pub fn add_translaas_from_env<S>(router: Router<S>) -> Result<Router<S>, InstallError>
where
    S: Clone + Send + Sync + 'static,
{
    add_translaas(router, |_| InstallOptions::from_env())
}

fn install_error_from_service(err: crate::service::Error) -> InstallError {
    match err {
        crate::service::Error::Client(crate::client::Error::Configuration(inner)) => {
            if inner.message.contains("ApiKey") {
                InstallError::MissingApiKey
            } else {
                InstallError::Configuration(inner)
            }
        }
        other => InstallError::Configuration(ConfigurationError {
            message: other.to_string(),
        }),
    }
}

fn options_from_map(get: impl Fn(&str) -> Option<String>) -> Result<InstallOptions, InstallError> {
    let api_key = match get("TRANSLAAS_API_KEY") {
        Some(value) if !value.trim().is_empty() => value,
        _ => return Err(InstallError::MissingApiKey),
    };

    let base_url = get("TRANSLAAS_BASE_URL")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
    let project = get("TRANSLAAS_DEFAULT_PROJECT")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROJECT.to_string());
    let language = get("TRANSLAAS_DEFAULT_LANGUAGE")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_LANGUAGE.to_string());
    let tls_flag = get("TRANSLAAS_TLS_INSECURE");

    let mut opts = InstallOptions::new()
        .api_key(api_key)
        .base_url(&base_url)
        .default_project_id(project)
        .default_language(language)
        .accept_invalid_certs(tls_insecure_enabled(&base_url, tls_flag.as_deref()));

    #[cfg(feature = "cache")]
    if let Some(raw) = get("TRANSLAAS_CACHE_MODE").filter(|value| !value.trim().is_empty()) {
        opts = opts.cache_mode(parse_cache_mode(&raw)?);
    }

    Ok(opts)
}

fn tls_insecure_enabled(base_url: &str, tls_insecure: Option<&str>) -> bool {
    if base_url.contains(".translaas.local") {
        return true;
    }
    matches!(tls_insecure, Some("1") | Some("true") | Some("TRUE"))
}

#[cfg(feature = "cache")]
fn parse_cache_mode(raw: &str) -> Result<CacheMode, InstallError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(CacheMode::None),
        "entry" => Ok(CacheMode::Entry),
        "group" => Ok(CacheMode::Group),
        "project" => Ok(CacheMode::Project),
        _ => Err(InstallError::Env {
            variable: "TRANSLAAS_CACHE_MODE",
            message: format!("invalid value '{raw}'; expected none, entry, group, or project"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| {
            pairs
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
        }
    }

    #[test]
    fn from_env_missing_api_key_fails() {
        let err = options_from_map(env(&[])).unwrap_err();
        assert!(err.is_missing_api_key());
        assert!(err.to_string().contains("ApiKey"));
    }

    #[test]
    fn from_env_whitespace_api_key_fails() {
        let err = options_from_map(env(&[("TRANSLAAS_API_KEY", "  ")])).unwrap_err();
        assert!(err.is_missing_api_key());
    }

    #[test]
    fn from_env_applies_defaults() {
        let opts = options_from_map(env(&[("TRANSLAAS_API_KEY", "test-key")])).unwrap();
        assert_eq!(opts.api_key, "test-key");
        assert_eq!(opts.base_url, DEFAULT_BASE_URL);
        assert_eq!(opts.default_project_id.as_deref(), Some(DEFAULT_PROJECT));
        assert_eq!(opts.default_language.as_deref(), Some(DEFAULT_LANGUAGE));
        assert!(opts.accept_invalid_certs);
    }

    #[test]
    fn tls_helper_matches_translaas_local_and_flag() {
        assert!(tls_insecure_enabled("https://api.translaas.local", None));
        assert!(!tls_insecure_enabled("https://api.example.com", None));
        assert!(tls_insecure_enabled("https://api.example.com", Some("1")));
        assert!(tls_insecure_enabled(
            "https://api.example.com",
            Some("true")
        ));
        assert!(tls_insecure_enabled(
            "https://api.example.com",
            Some("TRUE")
        ));
        assert!(!tls_insecure_enabled("https://api.example.com", Some("0")));
    }

    #[test]
    #[cfg(feature = "cache")]
    fn from_env_parses_cache_mode() {
        let opts = options_from_map(env(&[
            ("TRANSLAAS_API_KEY", "k"),
            ("TRANSLAAS_CACHE_MODE", "Group"),
        ]))
        .unwrap();
        assert_eq!(opts.cache_mode, Some(CacheMode::Group));
    }

    #[test]
    #[cfg(feature = "cache")]
    fn from_env_rejects_invalid_cache_mode() {
        let err = options_from_map(env(&[
            ("TRANSLAAS_API_KEY", "k"),
            ("TRANSLAAS_CACHE_MODE", "banana"),
        ]))
        .unwrap_err();
        match err {
            InstallError::Env { variable, message } => {
                assert_eq!(variable, "TRANSLAAS_CACHE_MODE");
                assert!(message.contains("banana"));
            }
            other => panic!("expected Env, got {other:?}"),
        }
    }

    #[test]
    fn empty_api_key_fails_before_build() {
        let err = InstallOptions::new()
            .api_key("  ")
            .base_url("https://example.com")
            .into_builder()
            .unwrap_err();
        assert!(err.is_missing_api_key());
    }
}
