//! Official Translaas client SDK for Rust.
//!
//! This crate ships shared models (`translaas::models`) and will add HTTP client,
//! caching, and service helpers in later releases. Behavioral parity targets are
//! documented in the umbrella
//! [implementation plan](https://github.com/Mantelabs/translaas-all/blob/main/.docs/translaas-sdk-rust-implementation.md).

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
