//! Shared helpers for live API integration tests.

use std::time::Duration;

use tokio::sync::OnceCell;
use translaas::client::{Client, ClientBuilder, Error};
use translaas::models::ApiError;

use super::config::Config;

static REACHABILITY: OnceCell<bool> = OnceCell::const_new();

/// Returns configuration when live tests should run; otherwise `None` (test should return early).
pub async fn require_integration_config() -> Option<Config> {
    let cfg = Config::load();
    if !cfg.enabled {
        return None;
    }
    if !probe_api_reachable(&cfg).await {
        return None;
    }
    Some(cfg)
}

/// Prints a single suite-level skip reason (called from `a00_suite_precheck` only).
pub async fn print_suite_skip_reason() {
    let cfg = Config::load();
    if !cfg.enabled {
        println!("\nintegration tests disabled: set TRANSLAAS_API_KEY\n");
        return;
    }
    if !probe_api_reachable(&cfg).await {
        println!(
            "\nintegration API not reachable at {} — skipping live tests\n\
             hint: start a delivery API or set TRANSLAAS_BASE_URL \
             (local Docker profile `core` uses https://api.translaas.local)\n",
            cfg.base_url
        );
    }
}

/// Logs and returns `true` when a test should soft-skip due to missing fixture data.
pub fn soft_skip_if(condition: bool, message: &str) -> bool {
    if condition {
        eprintln!("skipping: {message}");
        true
    } else {
        false
    }
}

/// True when the delivery API reports a missing SDK resource (Mantelabs platform uses HTTP 404).
pub fn is_sdk_not_found(err: &Error) -> bool {
    err.as_api()
        .is_some_and(|api| api.status_code == 404)
}

/// Soft-skip when the configured project (or resource) is missing on the API.
pub fn soft_skip_on_sdk_not_found(err: &Error) -> bool {
    if !is_sdk_not_found(err) {
        return false;
    }
    soft_skip_if(
        true,
        "SDK resource not found (HTTP 404) — set TRANSLAAS_DEFAULT_PROJECT to an existing project id (default: translaas-sdk-samples)",
    )
}

/// Same as [`soft_skip_on_sdk_not_found`] for [`translaas::service::Error`].
pub fn soft_skip_on_service_sdk_not_found(err: &translaas::service::Error) -> bool {
    match err {
        translaas::service::Error::Client(e) => soft_skip_on_sdk_not_found(e),
        _ => false,
    }
}

/// Builds a client using integration defaults (30s timeout, default project).
pub async fn new_integration_client() -> Option<(Config, Client)> {
    let cfg = require_integration_config().await?;
    Some((cfg.clone(), build_client(&cfg, Duration::from_secs(30))))
}

pub fn new_client_with_options(
    cfg: &Config,
    api_key: &str,
    base_url: &str,
    timeout: Duration,
) -> Client {
    integration_client_builder(cfg, timeout)
        .api_key(api_key)
        .base_url(base_url)
        .build()
        .expect("integration client")
}

/// Starts a [`ClientBuilder`] configured for local integration tests (incl. self-signed TLS).
pub fn integration_client_builder(cfg: &Config, timeout: Duration) -> ClientBuilder {
    ClientBuilder::new()
        .default_project_id(&cfg.default_project)
        .timeout(timeout)
        .http_client(build_http_client(timeout))
}

fn build_client(cfg: &Config, timeout: Duration) -> Client {
    new_client_with_options(cfg, &cfg.api_key, &cfg.base_url, timeout)
}

fn build_http_client(timeout: Duration) -> reqwest::Client {
    reqwest::Client::builder()
        .use_rustls_tls()
        // Local Docker uses self-signed certs (see platform/translaas/docs/docker-https-setup.md).
        .danger_accept_invalid_certs(true)
        .timeout(timeout)
        .build()
        .expect("integration HTTP client")
}

async fn probe_api_reachable(cfg: &Config) -> bool {
    let cfg = cfg.clone();
    *REACHABILITY
        .get_or_init(|| async move { do_probe(&cfg).await })
        .await
}

async fn do_probe(cfg: &Config) -> bool {
    let timeout = Duration::from_secs(5);
    let client = match integration_client_builder(cfg, timeout)
        .api_key(cfg.api_key.trim())
        .base_url(cfg.base_url.trim())
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };

    match client.validate_api_key().await {
        Ok(_) => true,
        Err(Error::Api(api)) if api.status_code == 401 || api.status_code == 403 => true,
        Err(Error::Api(api)) if is_transport_failure(&api) => false,
        Err(err) => {
            let msg = err.to_string().to_lowercase();
            !(msg.contains("connection refused")
                || msg.contains("no such host")
                || msg.contains("actively refused")
                || msg.contains("dns error")
                || msg.contains("failed to connect")
                || msg.contains("error sending request"))
        }
    }
}

fn is_transport_failure(api: &ApiError) -> bool {
    let msg = api.message.as_deref().unwrap_or("").to_lowercase();
    msg.contains("error sending request")
        || msg.contains("connection refused")
        || msg.contains("no such host")
        || msg.contains("actively refused")
        || msg.contains("dns error")
        || msg.contains("failed to connect")
}
