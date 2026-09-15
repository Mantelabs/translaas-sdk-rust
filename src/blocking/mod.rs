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

mod runtime;

#[allow(unused_imports)] // used by Client/Service wrappers
pub(crate) use runtime::BlockingDriver;
