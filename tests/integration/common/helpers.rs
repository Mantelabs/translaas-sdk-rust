//! Shared helpers for live API integration tests.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use translaas::client::{Client, ClientBuilder, Error};
use translaas::models::ApiError;

use super::config::Config;

static REACHABILITY: OnceLock<Mutex<Option<bool>>> = OnceLock::new();

/// Returns configuration when live tests should run; otherwise `None` (test should return early).
pub async fn require_integration_config() -> Option<Config> {
    let cfg = Config::load();
    if !cfg.enabled {
        eprintln!("integration tests disabled: set TRANSLAAS_API_KEY");
        return None;
    }
    if !probe_api_reachable(&cfg).await {
        eprintln!("integration API not reachable at {}", cfg.base_url);
        return None;
    }
    Some(cfg)
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
    ClientBuilder::new()
        .api_key(api_key)
        .base_url(base_url)
        .default_project_id(&cfg.default_project)
        .timeout(timeout)
        .build()
        .expect("integration client")
}

fn build_client(cfg: &Config, timeout: Duration) -> Client {
    new_client_with_options(cfg, &cfg.api_key, &cfg.base_url, timeout)
}

async fn probe_api_reachable(cfg: &Config) -> bool {
    let lock = REACHABILITY.get_or_init(|| Mutex::new(None));
    if let Some(reachable) = *lock.lock().expect("reachability lock") {
        return reachable;
    }
    let reachable = do_probe(cfg).await;
    *lock.lock().expect("reachability lock") = Some(reachable);
    reachable
}

async fn do_probe(cfg: &Config) -> bool {
    let client = match ClientBuilder::new()
        .api_key(cfg.api_key.trim())
        .base_url(cfg.base_url.trim())
        .timeout(Duration::from_secs(5))
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
