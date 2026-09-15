//! Sync Translaas client and service (opt-in `blocking` Cargo feature).
//!
//! These APIs wrap the async [`crate::client::Client`] / [`crate::service::Service`]
//! with an internal current-thread Tokio runtime. Application crates do **not** need
//! a `tokio` dependency.
//!
//! # Safety
//!
//! Do **not** call blocking APIs from an async Tokio task or Axum handler. That
//! returns [`crate::client::Error::BlockingInAsyncContext`] instead of deadlocking.
//! Nested sync calls on the same driver are not supported (Tokio cannot nest
//! `block_on`); prefer a single `t()` / `get_entry()` per lookup.
//!
//! ```no_run
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let translaas = translaas::blocking::ClientBuilder::new()
//!         .api_key(std::env::var("TRANSLAAS_API_KEY")?)
//!         .base_url("https://api.translaas.local")
//!         .default_project_id("translaassdksamples")
//!         .default_language("en")
//!         .accept_invalid_certs(true) // DEV ONLY
//!         .build_service()?;
//!     let text = translaas.t("common", "welcome.message")?;
//!     println!("{text}");
//!     Ok(())
//! }
//! ```

mod builder;
mod client;
mod runtime;
mod service;

pub use builder::ClientBuilder;
pub use client::Client;
pub use service::Service;

pub(crate) use runtime::BlockingDriver;
