//! Official Translaas client SDK for Rust.
//!
//! This crate ships shared models (`translaas::models`) and a live HTTP
//! [`client`] with text, group, project, locales, offline ZIP, report-missing,
//! and validate-api-key calls. Caching and service helpers arrive in later
//! releases. Behavioral parity targets are documented in the umbrella
//! [porting reference](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-dotnet-porting-reference.md).
//!
//! Callers need an async runtime (for example Tokio) to drive [`client::Client`] methods.

#![forbid(unsafe_code)]

pub mod models;

#[cfg(feature = "cache")]
pub mod cache;

pub mod client;

#[cfg(feature = "offline")]
pub mod cachefile;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "axum")]
pub mod axum;

mod http;
mod validate;
