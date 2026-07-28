//! Options for [`super::Service`] and [`super::Service::t`].

use std::collections::HashMap;

use crate::models::RequestContext;

use super::language::LanguageContext;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum LangChoice {
    #[default]
    Auto,
    Explicit(String),
}

/// Configures [`super::Service`] construction.
#[derive(Debug, Default, Clone)]
pub struct ServiceOptions {
    /// Optional language resolver for automatic locale selection.
    pub resolver: Option<super::language::LanguageResolver>,
}

/// Per-call options for [`super::Service::t`].
#[derive(Debug)]
pub struct TOptions<'a> {
    lang: LangChoice,
    pub(crate) number: Option<f64>,
    pub(crate) parameters: HashMap<String, String>,
    pub(crate) request_context: Option<&'a mut RequestContext>,
    pub(crate) language_context: LanguageContext,
}

impl<'a> TOptions<'a> {
    /// Creates options with automatic language resolution.
    pub fn new() -> Self {
        Self {
            lang: LangChoice::Auto,
            number: None,
            parameters: HashMap::new(),
            request_context: None,
            language_context: LanguageContext::new(),
        }
    }

    /// Sets an explicit language. Non-empty values bypass the resolver; empty or
    /// whitespace-only values trigger automatic resolution.
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = LangChoice::Explicit(lang.into());
        self
    }

    /// Sets the plural / interpolation count forwarded to `get_entry`.
    pub fn number(mut self, number: f64) -> Self {
        self.number = Some(number);
        self
    }

    /// Sets interpolation query parameters forwarded to `get_entry`.
    pub fn parameters(mut self, parameters: HashMap<String, String>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the mutable request context forwarded to `get_entry`.
    pub fn request_context(mut self, request_context: &'a mut RequestContext) -> Self {
        self.request_context = Some(request_context);
        self
    }

    /// Sets request-scoped language hints for resolver providers.
    pub fn language_context(mut self, language_context: LanguageContext) -> Self {
        self.language_context = language_context;
        self
    }

    pub(crate) fn explicit_lang_bypass(&self) -> Option<&str> {
        match &self.lang {
            LangChoice::Auto => None,
            LangChoice::Explicit(lang) if lang.trim().is_empty() => None,
            LangChoice::Explicit(lang) => Some(lang.as_str()),
        }
    }
}

impl<'a> Default for TOptions<'a> {
    fn default() -> Self {
        Self::new()
    }
}
