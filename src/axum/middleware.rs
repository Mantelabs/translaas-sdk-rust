//! Axum middleware for request-scoped Translaas services.

use std::sync::Arc;

use axum::extract::State;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::service::{LanguageProvider, Service};

use super::error::MiddlewareError;
use super::language::{
    default_language_sources, language_context_from_parts, LanguageSource, RequestLanguageOptions,
    RequestLanguageProvider, RouteLanguageFn,
};

/// Configures request-scoped service injection.
#[derive(Clone)]
pub struct MiddlewareOptions<C> {
    /// Base service shared across requests.
    pub base_service: Option<Service<C>>,
    /// Ordered language sources. Defaults to [`default_language_sources`].
    pub language_sources: Option<Vec<LanguageSource>>,
    /// Names for query/header/cookie/route extraction.
    pub request_language: RequestLanguageOptions,
    /// Optional route language resolver for [`LanguageSource::Route`].
    pub route_language: Option<RouteLanguageFn>,
}

impl<C> MiddlewareOptions<C> {
    /// Returns options with common language source defaults.
    pub fn with_base_service(base_service: Service<C>) -> Self {
        Self {
            base_service: Some(base_service),
            language_sources: None,
            request_language: RequestLanguageOptions::default(),
            route_language: None,
        }
    }
}

/// Shared state for [`translaas_middleware`].
#[derive(Clone)]
pub struct TranslaasMiddlewareState<C> {
    pub(crate) base_service: Service<C>,
    pub(crate) language_sources: Vec<LanguageSource>,
    pub(crate) request_language: RequestLanguageOptions,
    pub(crate) route_language: Option<RouteLanguageFn>,
}

impl<C> TryFrom<MiddlewareOptions<C>> for TranslaasMiddlewareState<C> {
    type Error = MiddlewareError;

    fn try_from(opts: MiddlewareOptions<C>) -> Result<Self, Self::Error> {
        middleware(opts)
    }
}

/// Validates options and returns middleware state (Go `web.Middleware` parity).
pub fn middleware<C>(
    opts: MiddlewareOptions<C>,
) -> Result<TranslaasMiddlewareState<C>, MiddlewareError> {
    let base_service = opts
        .base_service
        .ok_or(MiddlewareError::MissingBaseService)?;

    let language_sources = opts
        .language_sources
        .unwrap_or_else(default_language_sources);

    Ok(TranslaasMiddlewareState {
        base_service,
        language_sources,
        request_language: opts.request_language,
        route_language: opts.route_language,
    })
}

/// Injects a request-scoped [`Service`] and [`LanguageContext`](crate::service::LanguageContext).
///
/// ```no_run
/// # use std::sync::Arc;
/// # use translaas::axum::{middleware, MiddlewareOptions, translaas_middleware};
/// # use translaas::service::{DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions};
/// # fn demo(client: translaas::client::Client) -> Result<(), translaas::axum::MiddlewareError> {
/// let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")]).unwrap();
/// let base = Service::new(client, ServiceOptions { resolver: Some(resolver) });
/// let state = Arc::new(middleware(MiddlewareOptions::with_base_service(base))?);
/// // Router::new().layer(axum::middleware::from_fn_with_state(state.clone(), translaas_middleware))
/// # let _ = state;
/// # Ok(())
/// # }
/// ```
pub async fn translaas_middleware<C>(
    State(state): State<Arc<TranslaasMiddlewareState<C>>>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response
where
    C: crate::client::TranslaasClient + Clone + Send + Sync + 'static,
{
    let (parts, body) = req.into_parts();
    let req_provider = RequestLanguageProvider::from_parts(
        &parts,
        state.request_language.clone(),
        state.language_sources.clone(),
        state.route_language.clone(),
    );

    let resolved_language = req_provider
        .language(&crate::service::LanguageContext::new())
        .unwrap_or(None);

    let request_service = match state.base_service.with_prepended_providers([req_provider]) {
        Ok(service) => service,
        Err(_) => {
            return (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "translaas middleware misconfigured",
            )
                .into_response();
        }
    };

    let language_context = language_context_from_parts(&parts, resolved_language);
    let mut req = Request::from_parts(parts, body);
    req.extensions_mut().insert(request_service);
    req.extensions_mut().insert(language_context);

    next.run(req).await
}

use axum::response::IntoResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn middleware_requires_base_service() {
        let result = middleware(MiddlewareOptions::<()> {
            base_service: None,
            language_sources: None,
            request_language: RequestLanguageOptions::default(),
            route_language: None,
        });

        assert!(matches!(result, Err(MiddlewareError::MissingBaseService)));
    }
}
