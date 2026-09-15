//! Current-thread Tokio runtime that drives async client/service futures.

use std::cell::Cell;
use std::future::Future;
use std::sync::Arc;

use tokio::runtime::{Builder, Handle, Runtime};

use crate::client::Error;
use crate::models::ConfigurationError;

thread_local! {
    static ENTERED: Cell<usize> = const { Cell::new(0) };
}

/// Owns a current-thread runtime used exclusively by blocking wrappers.
#[derive(Clone)]
pub(crate) struct BlockingDriver {
    runtime: Arc<Runtime>,
}

impl BlockingDriver {
    pub(crate) fn new() -> Result<Self, ConfigurationError> {
        let runtime = Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|err| ConfigurationError {
                message: format!("failed to start blocking runtime: {err}"),
            })?;
        Ok(Self {
            runtime: Arc::new(runtime),
        })
    }

    /// Runs `fut` on this driver, or returns [`Error::BlockingInAsyncContext`].
    ///
    /// Rejects a **foreign** Tokio context (`Handle::try_current` while we are not
    /// already inside this driver). Also rejects re-entry: Tokio cannot nest
    /// `Runtime::block_on` on the same thread.
    pub(crate) fn block_on<T>(&self, fut: impl Future<Output = T>) -> Result<T, Error> {
        if ENTERED.with(Cell::get) > 0 {
            return Err(Error::BlockingInAsyncContext);
        }
        if Handle::try_current().is_ok() {
            return Err(Error::BlockingInAsyncContext);
        }

        ENTERED.with(|entered| entered.set(entered.get() + 1));
        let output = self.runtime.block_on(fut);
        ENTERED.with(|entered| entered.set(entered.get() - 1));
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn driver_builds() {
        BlockingDriver::new().expect("current-thread runtime");
    }

    #[test]
    fn block_on_plain_thread_runs_future() {
        let driver = BlockingDriver::new().unwrap();
        let value = driver.block_on(async { 7 }).unwrap();
        assert_eq!(value, 7);
    }

    #[tokio::test]
    async fn block_on_rejects_foreign_runtime() {
        let driver = std::thread::spawn(BlockingDriver::new)
            .join()
            .expect("spawn")
            .expect("runtime");
        let err = driver.block_on(async { 1 }).expect_err("foreign runtime");
        assert!(err.is_blocking_in_async_context());
        assert!(!err.is_transport());
        assert!(err.as_api().is_none());
        std::thread::spawn(move || drop(driver))
            .join()
            .expect("drop runtime off async thread");
    }
}
