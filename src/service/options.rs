//! Options for [`super::Service`], [`super::Service::t`], [`super::Service::t_lang`],
//! [`super::Service::t_params`], and [`super::Service::t_with`].

use std::collections::HashMap;

use crate::models::RequestContext;

use super::language::LanguageContext;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum LangChoice {
    #[default]
    Auto,
    Explicit(String),
}

/// Interpolation extras for [`super::Service::t_params`]: named `{placeholders}` and/or plural `{N}`.
/// Language is optional — omit it to use `default_language` / the resolver (same as [`super::Service::t`]).
///
/// Typical call sites:
///
/// ```ignore
/// translaas.t_params("messages", "hello", [("name", "Ada")]).await?;
/// translaas.t_params("messages", "item", 5).await?;
/// translaas.t_params("messages", "hello", TParams::new().lang("de").param("name", "Ada")).await?;
/// ```
#[derive(Debug, Default, Clone)]
pub struct TParams {
    lang: LangChoice,
    pub(crate) number: Option<f64>,
    pub(crate) parameters: HashMap<String, String>,
}

impl TParams {
    /// Empty extras (auto language, no `{N}`, no named placeholders).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an explicit language. Non-empty values bypass the resolver; empty or
    /// whitespace-only values trigger automatic resolution (same as [`TOptions::lang`]).
    pub fn lang(mut self, lang: impl Into<String>) -> Self {
        self.lang = LangChoice::Explicit(lang.into());
        self
    }

    /// Sets the plural / interpolation count (`{N}`).
    pub fn number(mut self, number: f64) -> Self {
        self.number = Some(number);
        self
    }

    /// Adds or replaces one named placeholder (`{name}` in the catalog).
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Replaces all named placeholders. Accepts `[("name", "Ada")]` or a [`HashMap`].
    pub fn parameters<I, K, V>(mut self, parameters: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.parameters = parameters
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
        self
    }

    pub(crate) fn apply_to<'a>(self, mut opts: TOptions<'a>) -> TOptions<'a> {
        if let LangChoice::Explicit(lang) = self.lang {
            opts = opts.lang(lang);
        }
        if let Some(number) = self.number {
            opts = opts.number(number);
        }
        opts.parameters(self.parameters)
    }
}

impl From<i32> for TParams {
    fn from(number: i32) -> Self {
        TParams::new().number(f64::from(number))
    }
}

impl From<f64> for TParams {
    fn from(number: f64) -> Self {
        TParams::new().number(number)
    }
}

impl<K, V, const N: usize> From<[(K, V); N]> for TParams
where
    K: Into<String>,
    V: Into<String>,
{
    fn from(pairs: [(K, V); N]) -> Self {
        TParams::new().parameters(pairs)
    }
}

impl<K, V, const N: usize> From<(i32, [(K, V); N])> for TParams
where
    K: Into<String>,
    V: Into<String>,
{
    fn from((number, pairs): (i32, [(K, V); N])) -> Self {
        TParams::from(number).parameters(pairs)
    }
}

impl<K, V, const N: usize> From<(f64, [(K, V); N])> for TParams
where
    K: Into<String>,
    V: Into<String>,
{
    fn from((number, pairs): (f64, [(K, V); N])) -> Self {
        TParams::from(number).parameters(pairs)
    }
}

impl<L> From<(L, i32)> for TParams
where
    L: Into<String>,
{
    fn from((lang, number): (L, i32)) -> Self {
        TParams::from(number).lang(lang)
    }
}

impl<L> From<(L, f64)> for TParams
where
    L: Into<String>,
{
    fn from((lang, number): (L, f64)) -> Self {
        TParams::from(number).lang(lang)
    }
}

/// Configures [`super::Service`] construction.
#[derive(Debug, Default, Clone)]
pub struct ServiceOptions {
    /// Optional language resolver for automatic locale selection.
    pub resolver: Option<super::language::LanguageResolver>,
}

/// Per-call options for [`super::Service::t_with`].
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

    /// Adds or replaces one interpolation parameter (`{name}` in the catalog).
    ///
    /// Chain for several keys: `.param("name", "Ada").param("city", "Lima")`.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(key.into(), value.into());
        self
    }

    /// Sets interpolation query parameters forwarded to `get_entry`.
    ///
    /// Accepts a [`HashMap`] or a list of pairs, for example `[("name", "Ada")]`.
    /// Replaces any parameters previously set with [`Self::param`].
    pub fn parameters<I, K, V>(mut self, parameters: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.parameters = parameters
            .into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect();
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
