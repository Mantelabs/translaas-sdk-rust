//! Convenience `t()` API with automatic language resolution.
//!
//! Wrap [`TranslaasClient`](crate::client::TranslaasClient) with a provider chain for
//! locale selection, then delegate to `get_entry`.
//!
//! # Quick start
//!
//! ```no_run
//! # async fn example() -> Result<(), translaas::service::Error> {
//! use translaas::client::{Client, ClientBuilder};
//! use translaas::service::{
//!     DefaultLanguageProvider, LanguageResolver, Service, ServiceOptions, TOptions,
//! };
//!
//! let client = ClientBuilder::new()
//!     .base_url("https://api.example.com")
//!     .api_key("key")
//!     .build()?;
//!
//! let resolver = LanguageResolver::new([DefaultLanguageProvider::new("en")])?;
//! let service = Service::new(client, ServiceOptions {
//!     resolver: Some(resolver),
//! });
//!
//! let text = service
//!     .t("common", "welcome", TOptions::new().lang("de"))
//!     .await?;
//! # let _ = text;
//! # Ok(())
//! # }
//! ```

mod error;
mod language;
mod options;
mod translation;

pub use error::Error;
pub use options::{ServiceOptions, TOptions};
pub use translation::Service;

pub use language::{
    normalize_language_code, parse_accept_language, AcceptLanguageProvider,
    ContextLanguageProvider, DefaultLanguageProvider, LanguageContext, LanguageProvider,
    LanguageProviderError, LanguageResolver, LanguageResolverError,
};
