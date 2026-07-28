//! Axum extractors for request-scoped Translaas services.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::service::{Error, LanguageContext, Service, TOptions};

use super::error::TranslaasRejection;

/// Extracts the request-scoped [`Service`] injected by [`super::middleware::translaas_middleware`].
#[derive(Clone)]
pub struct Translaas<C>(pub Service<C>);

impl<C> std::fmt::Debug for Translaas<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Translaas").field(&self.0).finish()
    }
}

impl<C> Translaas<C>
where
    C: crate::client::TranslaasClient,
{
    /// Returns the underlying service.
    pub fn into_inner(self) -> Service<C> {
        self.0
    }

    /// Returns a reference to the underlying service.
    pub fn service(&self) -> &Service<C> {
        &self.0
    }

    /// Calls [`Service::t`] merging the request [`LanguageContext`] from extensions when present.
    pub async fn t(
        &self,
        parts: &Parts,
        group: &str,
        entry: &str,
        mut opts: TOptions<'_>,
    ) -> Result<String, Error> {
        if let Some(ctx) = parts.extensions.get::<LanguageContext>() {
            opts = opts.language_context(ctx.clone());
        }
        self.0.t(group, entry, opts).await
    }
}

impl<C, S> FromRequestParts<S> for Translaas<C>
where
    C: crate::client::TranslaasClient + Clone + Send + Sync + 'static,
    S: Send + Sync,
{
    type Rejection = TranslaasRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Service<C>>()
            .cloned()
            .map(Translaas)
            .ok_or(TranslaasRejection::MissingService)
    }
}

/// Extracts the request [`LanguageContext`] injected by middleware.
#[derive(Debug, Clone)]
pub struct LanguageContextExt(pub LanguageContext);

impl<S> FromRequestParts<S> for LanguageContextExt
where
    S: Send + Sync,
{
    type Rejection = TranslaasRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<LanguageContext>()
            .cloned()
            .map(LanguageContextExt)
            .ok_or(TranslaasRejection::MissingService)
    }
}
