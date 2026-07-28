//! Language provider trait and built-in implementations.

use std::sync::Arc;

use super::accept_language::{normalize_language_code, parse_accept_language};
use super::context::LanguageContext;

/// Resolves a language code from request-scoped context.
pub trait LanguageProvider: Send + Sync {
    /// Returns a language code or `None` when this provider has no value.
    fn language(&self, ctx: &LanguageContext) -> Result<Option<String>, LanguageProviderError>;
}

/// A provider failed; the resolver continues to the next provider.
#[derive(Debug)]
pub struct LanguageProviderError {
    source: Box<dyn std::error::Error + Send + Sync>,
}

impl LanguageProviderError {
    /// Wraps an arbitrary provider failure.
    pub fn new(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

impl std::fmt::Display for LanguageProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "language provider failed: {}", self.source)
    }
}

impl std::error::Error for LanguageProviderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Always yields a configured default language.
pub struct DefaultLanguageProvider {
    lang: String,
}

impl DefaultLanguageProvider {
    /// Creates a provider that always yields the configured language.
    pub fn new(lang: impl Into<String>) -> Self {
        let raw = lang.into();
        let normalized = normalize_language_code(&raw);
        let lang = if normalized.is_empty() {
            raw.trim().to_ascii_lowercase()
        } else {
            normalized
        };
        Self { lang }
    }
}

impl LanguageProvider for DefaultLanguageProvider {
    fn language(&self, _ctx: &LanguageContext) -> Result<Option<String>, LanguageProviderError> {
        if self.lang.is_empty() {
            return Ok(None);
        }
        Ok(Some(self.lang.clone()))
    }
}

/// Reads an explicit language from [`LanguageContext`].
#[derive(Debug, Default, Clone, Copy)]
pub struct ContextLanguageProvider;

impl LanguageProvider for ContextLanguageProvider {
    fn language(&self, ctx: &LanguageContext) -> Result<Option<String>, LanguageProviderError> {
        let Some(lang) = ctx.language() else {
            return Ok(None);
        };

        let normalized = normalize_language_code(lang);
        if !normalized.is_empty() {
            return Ok(Some(normalized));
        }

        let fallback = lang.trim().to_ascii_lowercase();
        if fallback.is_empty() {
            Ok(None)
        } else {
            Ok(Some(fallback))
        }
    }
}

/// Parses `Accept-Language` from [`LanguageContext`].
#[derive(Debug, Default, Clone, Copy)]
pub struct AcceptLanguageProvider;

impl LanguageProvider for AcceptLanguageProvider {
    fn language(&self, ctx: &LanguageContext) -> Result<Option<String>, LanguageProviderError> {
        let Some(header) = ctx.accept_language() else {
            return Ok(None);
        };

        let parsed = parse_accept_language(header);
        if parsed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parsed))
        }
    }
}

/// Type-erased provider handle for resolver chains.
pub(crate) type DynLanguageProvider = Arc<dyn LanguageProvider>;

pub(crate) fn into_provider<P: LanguageProvider + 'static>(provider: P) -> DynLanguageProvider {
    Arc::new(provider)
}
