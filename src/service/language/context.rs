//! Request-scoped language hints for resolver providers.

/// Request-scoped language hints passed into resolver providers.
///
/// Replaces Go's `context.Context` value bag (`WithLanguage` / `WithAcceptLanguage`).
/// Axum helpers (#13) will construct this per request.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LanguageContext {
    language: Option<String>,
    accept_language: Option<String>,
}

impl LanguageContext {
    /// Creates an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an explicit language code for [`ContextLanguageProvider`](super::ContextLanguageProvider).
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Sets a raw `Accept-Language` header value for [`AcceptLanguageProvider`](super::AcceptLanguageProvider).
    pub fn with_accept_language(mut self, header: impl Into<String>) -> Self {
        self.accept_language = Some(header.into());
        self
    }

    /// Returns the explicit language, if any.
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Returns the raw `Accept-Language` header value, if any.
    pub fn accept_language(&self) -> Option<&str> {
        self.accept_language.as_deref()
    }
}
