//! Internal URL building and query encoding helpers.
//!
//! This module is crate-private and mirrors Go `internal/httpx` / .NET query
//! construction rules documented in the porting reference (section 5).
#![allow(dead_code, unused_imports)] // re-exports consumed by `client` in issue #4

mod query;
mod url;

#[cfg(test)]
mod golden;

pub(crate) use query::{append_query_values, inject_plural_n, merge_query_params, query_values};
pub(crate) use url::build_url;
