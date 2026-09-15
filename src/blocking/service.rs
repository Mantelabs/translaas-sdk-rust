//! Blocking wrapper around [`crate::service::Service`].

use crate::service::{TOptions, TParams};

use super::BlockingDriver;

/// Sync convenience `t()` API. Do not call from an async Tokio task.
pub struct Service {
    inner: crate::service::Service<crate::client::Client>,
    driver: BlockingDriver,
}

impl Service {
    pub(crate) fn from_parts(
        inner: crate::service::Service<crate::client::Client>,
        driver: BlockingDriver,
    ) -> Self {
        Self { inner, driver }
    }

    fn run<T>(
        &self,
        fut: impl std::future::Future<Output = Result<T, crate::service::Error>>,
    ) -> Result<T, crate::service::Error> {
        self.driver.block_on(fut)?
    }

    /// Retrieves a translation using automatic language resolution.
    pub fn t(&self, group: &str, entry: &str) -> Result<String, crate::service::Error> {
        self.run(self.inner.t(group, entry))
    }

    /// Retrieves a translation with an explicit language.
    pub fn t_lang(
        &self,
        group: &str,
        entry: &str,
        lang: impl Into<String>,
    ) -> Result<String, crate::service::Error> {
        self.run(self.inner.t_lang(group, entry, lang))
    }

    /// Retrieves a translation with named parameters and/or a plural count.
    pub fn t_params(
        &self,
        group: &str,
        entry: &str,
        params: impl Into<TParams>,
    ) -> Result<String, crate::service::Error> {
        self.run(self.inner.t_params(group, entry, params))
    }

    /// Retrieves a translation with extras (plural count, parameters, request context).
    pub fn t_with(
        &self,
        group: &str,
        entry: &str,
        opts: TOptions<'_>,
    ) -> Result<String, crate::service::Error> {
        self.run(self.inner.t_with(group, entry, opts))
    }
}
