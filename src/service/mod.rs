//! Convenience `t()` API with automatic language resolution.
//!
//! Wrap [`TranslaasClient`](crate::client::TranslaasClient) with a provider chain for
//! locale selection, then delegate to `get_entry`.
//!
//! # Quick start
//!
//! ```no_run
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use translaas::{ClientBuilder, Service};
//!
//! let translaas: Service<_> = ClientBuilder::new()
//!     .base_url("https://api.example.com")
//!     .api_key("key")
//!     .default_project_id("my-project")
//!     .default_language("en")
//!     .build_service()?;
//!
//! let text = translaas.t_lang("common", "welcome", "de").await?;
//! # let _ = text;
//! # Ok(())
//! # }
//! ```

mod client_builder;
mod error;
mod language;
mod options;
mod translation;

pub use error::Error;
pub use options::{ServiceOptions, TOptions, TParams};
pub use translation::Service;

pub use language::{
    normalize_language_code, parse_accept_language, AcceptLanguageProvider,
    ContextLanguageProvider, DefaultLanguageProvider, LanguageContext, LanguageProvider,
    LanguageProviderError, LanguageResolver, LanguageResolverError,
};
