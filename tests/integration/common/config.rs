//! Environment configuration for live API integration tests.

use std::env;

/// Default delivery API origin for local Docker (`platform/translaas` profile `core`).
/// Go SDK integration tests use `https://sdk-api.translaas.local`; override with
/// `TRANSLAAS_BASE_URL` when your environment differs.
/// Fixture ids aligned with [translaas-sdk-examples](https://github.com/Mantelabs/translaas-sdk-examples)
/// (`dotnet/docs/translaas_sdk_samples_strings.csv`, Java `TranslaasWebAppParity`).
/// Go / .NET *integration test* repos still document legacy `test-project` / `ui` / `button.save`
/// for generic dev APIs; local Mantelabs Docker uses `translaas-sdk-samples` instead.
pub const DEFAULT_BASE_URL: &str = "https://api.translaas.local";
pub const DEFAULT_PROJECT: &str = "translaas-sdk-samples";
pub const FIXTURE_GROUP: &str = "common";
pub const FIXTURE_GROUP_MESSAGES: &str = "messages";
pub const FIXTURE_ENTRY_SAVE: &str = "welcome.message";
pub const FIXTURE_ENTRY_PLURAL: &str = "item";
pub const FIXTURE_LANG: &str = "en";

/// Integration test environment settings.
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub default_project: String,
    pub enabled: bool,
}

impl Config {
    /// Loads configuration from environment variables.
    pub fn load() -> Self {
        let api_key = env::var("TRANSLAAS_API_KEY").unwrap_or_default();
        let base_url =
            env::var("TRANSLAAS_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let default_project =
            env::var("TRANSLAAS_DEFAULT_PROJECT").unwrap_or_else(|_| DEFAULT_PROJECT.to_string());

        Self {
            enabled: !api_key.trim().is_empty(),
            api_key,
            base_url,
            default_project,
        }
    }
}
