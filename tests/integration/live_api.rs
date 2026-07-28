//! Live API integration tests (issue #14). Run via `make test-integration`.

mod common;
mod error_scenarios;
mod get_entry;
mod get_group;
mod get_project;
mod get_project_locales;
mod validate_api_key;

#[cfg(feature = "service")]
mod service;
