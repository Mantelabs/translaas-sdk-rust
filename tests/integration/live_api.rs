//! Live API integration tests (issue #14). Run via `make test-integration`.

mod common;

/// Runs reachability / env checks once before the rest of the suite (name sorts first).
#[tokio::test]
async fn a00_suite_precheck() {
    common::print_suite_skip_reason().await;
}

mod error_scenarios;
mod get_entry;
mod get_group;
mod get_project;
mod get_project_locales;
mod validate_api_key;

#[cfg(feature = "service")]
mod service;
