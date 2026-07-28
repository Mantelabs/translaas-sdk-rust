//! Language resolution for the convenience `t()` API.

mod accept_language;
mod context;
mod provider;
mod resolver;

pub use accept_language::{normalize_language_code, parse_accept_language};
pub use context::LanguageContext;
pub use provider::{
    AcceptLanguageProvider, ContextLanguageProvider, DefaultLanguageProvider, LanguageProvider,
    LanguageProviderError,
};
pub use resolver::{LanguageResolver, LanguageResolverError};
