//! Convenience translation service wrapping [`TranslaasClient`](crate::client::TranslaasClient).

use crate::client::{GetEntryOptions, TranslaasClient};
use crate::models::NoLanguageError;

use super::error::Error;
use super::language::{DefaultLanguageProvider, LanguageProvider, LanguageResolver};
use super::options::{ServiceOptions, TOptions};

/// Convenience translation API with optional automatic language resolution.
#[derive(Clone)]
pub struct Service<C> {
    client: C,
    resolver: Option<LanguageResolver>,
}

impl<C> std::fmt::Debug for Service<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Service")
            .field("resolver", &self.resolver.is_some())
            .finish_non_exhaustive()
    }
}

impl<C: TranslaasClient> Service<C> {
    /// Constructs a service wrapping any [`TranslaasClient`] implementation.
    ///
    /// When the client has [`TranslaasClient::default_language`], a
    /// [`DefaultLanguageProvider`] is attached automatically. For a custom resolver chain,
    /// use [`Self::with_options`].
    ///
    /// Share the inner client with other decorators by wrapping it in [`std::sync::Arc`]
    /// before constructing multiple wrappers around the same HTTP client.
    pub fn new(client: C) -> Self {
        Self::with_options(client, ServiceOptions::default())
    }

    /// Constructs a service with an explicit resolver (or none).
    ///
    /// If `options.resolver` is `None`, falls back to [`TranslaasClient::default_language`].
    pub fn with_options(client: C, options: ServiceOptions) -> Self {
        let resolver = match options.resolver {
            Some(resolver) => Some(resolver),
            None => resolver_from_default_language(&client),
        };
        Self { client, resolver }
    }

    /// Returns a new service sharing the client with request-scoped providers tried first.
    pub fn with_prepended_providers(
        &self,
        providers: impl IntoIterator<Item = impl LanguageProvider + 'static>,
    ) -> Result<Service<C>, Error>
    where
        C: Clone,
    {
        let resolver = match &self.resolver {
            Some(existing) => existing.prepend_providers(providers)?,
            None => LanguageResolver::new(providers)?,
        };

        Ok(Service {
            client: self.client.clone(),
            resolver: Some(resolver),
        })
    }

    /// Retrieves a translation using automatic language resolution.
    pub async fn t(&self, group: &str, entry: &str) -> Result<String, Error> {
        self.t_with(group, entry, TOptions::new()).await
    }

    /// Retrieves a translation with an explicit language.
    ///
    /// Non-empty `lang` bypasses the resolver. Empty or whitespace-only values fall back
    /// to automatic resolution (same as [`TOptions::lang`]).
    pub async fn t_lang(
        &self,
        group: &str,
        entry: &str,
        lang: impl Into<String>,
    ) -> Result<String, Error> {
        self.t_with(group, entry, TOptions::new().lang(lang)).await
    }

    /// Retrieves a translation with extras (plural count, parameters, request context).
    pub async fn t_with(
        &self,
        group: &str,
        entry: &str,
        mut opts: TOptions<'_>,
    ) -> Result<String, Error> {
        let lang = self.resolve_language(&opts)?;
        let get_opts = build_get_entry_options(&mut opts);
        Ok(self.client.get_entry(group, entry, &lang, get_opts).await?)
    }

    fn resolve_language(&self, opts: &TOptions<'_>) -> Result<String, NoLanguageError> {
        if let Some(lang) = opts.explicit_lang_bypass() {
            return Ok(lang.to_string());
        }

        let Some(resolver) = &self.resolver else {
            return Err(NoLanguageError);
        };

        resolver.resolve(&opts.language_context)
    }
}

fn resolver_from_default_language<C: TranslaasClient>(client: &C) -> Option<LanguageResolver> {
    let lang = client.default_language()?.trim();
    if lang.is_empty() {
        return None;
    }
    LanguageResolver::new([DefaultLanguageProvider::new(lang)]).ok()
}

fn build_get_entry_options<'a>(opts: &mut TOptions<'a>) -> GetEntryOptions<'a> {
    let mut get_opts = GetEntryOptions::new();
    if let Some(number) = opts.number {
        get_opts = get_opts.number(number);
    }
    if !opts.parameters.is_empty() {
        get_opts = get_opts.parameters(std::mem::take(&mut opts.parameters));
    }
    if let Some(request_context) = opts.request_context.take() {
        get_opts = get_opts.request_context(request_context);
    }
    get_opts
}
