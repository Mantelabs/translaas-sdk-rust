//! Environment configuration for live API integration tests.

use std::env;

pub const DEFAULT_BASE_URL: &str = "https://sdk-api.translaas.local";
pub const DEFAULT_PROJECT: &str = "test-project";
pub const FIXTURE_GROUP: &str = "ui";
pub const FIXTURE_ENTRY_SAVE: &str = "button.save";
pub const FIXTURE_ENTRY_COUNT: &str = "items.count";
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
