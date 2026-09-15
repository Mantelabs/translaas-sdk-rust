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
//!     let client = translaas::blocking::ClientBuilder::new()
//!         .api_key(std::env::var("TRANSLAAS_API_KEY")?)
//!         .base_url("https://api.translaas.local")
//!         .default_project_id("translaassdksamples")
//!         .build()?;
//!     let text = client.get_entry(
//!         "common",
//!         "welcome.message",
//!         "en",
//!         translaas::client::GetEntryOptions::new(),
//!     )?;
//!     println!("{text}");
//!     Ok(())
//! }
//! ```

mod builder;
mod client;
mod runtime;

pub use builder::ClientBuilder;
pub use client::Client;

pub(crate) use runtime::BlockingDriver;
