//! Optional Axum extractors and helpers.
//!
//! # Security
//!
//! Translation strings returned by the SDK are **not** HTML-escaped. When rendering HTML,
//! escape at the template layer (`askama`, `maud`, etc.) rather than concatenating raw
//! translation output into markup. JSON and API responses must be encoded at the
//! serializer layer.

mod error;
mod extract;
mod language;
mod middleware;

pub use error::{MiddlewareError, TranslaasRejection};
pub use extract::{LanguageContextExt, Translaas};
pub use language::{
    default_language_sources, language_context_from_parts, LanguageSource, RequestLanguageOptions,
    RequestLanguageProvider, RouteLanguageFn,
};
pub use middleware::{
    middleware, translaas_middleware, MiddlewareOptions, TranslaasMiddlewareState,
};
