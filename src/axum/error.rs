//! Errors for Axum integration helpers.

use std::fmt;

/// Middleware or extractor misconfiguration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiddlewareError {
    /// No base [`Service`](crate::service::Service) was supplied.
    MissingBaseService,
}

impl fmt::Display for MiddlewareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingBaseService => write!(f, "axum: base service is required"),
        }
    }
}

impl std::error::Error for MiddlewareError {}

/// Rejection when a handler expects an injected Translaas service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslaasRejection {
    /// [`translaas_middleware`](super::middleware::translaas_middleware) was not registered.
    MissingService,
}

impl fmt::Display for TranslaasRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingService => {
                write!(
                    f,
                    "translaas service not found in request extensions; register middleware"
                )
            }
        }
    }
}

impl axum::response::IntoResponse for TranslaasRejection {
    fn into_response(self) -> axum::response::Response {
        let status = axum::http::StatusCode::INTERNAL_SERVER_ERROR;
        (status, self.to_string()).into_response()
    }
}
