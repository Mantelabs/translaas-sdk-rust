//! Language resolver — first non-empty provider wins.

use thiserror::Error;

use crate::models::NoLanguageError;

use super::context::LanguageContext;
use super::provider::{
    into_provider, DynLanguageProvider, LanguageProvider, LanguageProviderError,
};

/// Resolver configuration error.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("language: at least one provider is required")]
pub struct LanguageResolverError;

/// Chains language providers; the first non-empty language wins.
#[derive(Clone)]
pub struct LanguageResolver {
    providers: Vec<DynLanguageProvider>,
}

impl std::fmt::Debug for LanguageResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageResolver")
            .field("providers", &self.providers.len())
            .finish()
    }
}

impl LanguageResolver {
    /// Builds a resolver from providers evaluated in registration order.
    pub fn new(
        providers: impl IntoIterator<Item = impl LanguageProvider + 'static>,
    ) -> Result<Self, LanguageResolverError> {
        Self::from_providers(providers.into_iter().map(into_provider).collect::<Vec<_>>())
    }

    /// Builds a resolver from type-erased providers (supports mixed provider types).
    pub fn from_providers(
        providers: Vec<DynLanguageProvider>,
    ) -> Result<Self, LanguageResolverError> {
        if providers.is_empty() {
            return Err(LanguageResolverError);
        }
        Ok(Self { providers })
    }

    /// Returns a new resolver that tries `providers` before the existing chain.
    pub fn prepend_providers(
        &self,
        providers: impl IntoIterator<Item = impl LanguageProvider + 'static>,
    ) -> Result<Self, LanguageResolverError> {
        let prepend: Vec<DynLanguageProvider> = providers.into_iter().map(into_provider).collect();

        if prepend.is_empty() {
            return Ok(self.clone());
        }

        let mut all = Vec::with_capacity(prepend.len() + self.providers.len());
        all.extend(prepend);
        all.extend(self.providers.iter().cloned());
        Ok(Self { providers: all })
    }

    /// Returns the first non-empty language from the provider chain.
    pub fn resolve(&self, ctx: &LanguageContext) -> Result<String, NoLanguageError> {
        for provider in &self.providers {
            match provider.language(ctx) {
                Ok(Some(lang)) if !lang.trim().is_empty() => return Ok(lang),
                Ok(_) | Err(LanguageProviderError { .. }) => continue,
            }
        }
        Err(NoLanguageError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::language::{
        AcceptLanguageProvider, ContextLanguageProvider, DefaultLanguageProvider,
    };

    struct StubProvider {
        lang: String,
        fail: bool,
    }

    impl LanguageProvider for StubProvider {
        fn language(
            &self,
            _ctx: &LanguageContext,
        ) -> Result<Option<String>, LanguageProviderError> {
            if self.fail {
                return Err(LanguageProviderError::new(std::io::Error::other("boom")));
            }
            Ok(if self.lang.is_empty() {
                None
            } else {
                Some(self.lang.clone())
            })
        }
    }

    #[test]
    fn new_resolver_requires_providers() {
        let err = LanguageResolver::from_providers(vec![]).unwrap_err();
        assert_eq!(err, LanguageResolverError);
    }

    #[test]
    fn resolver_order_context_over_accept_over_default() {
        let ctx = LanguageContext::new()
            .with_language("fr")
            .with_accept_language("en-US,en;q=0.9");

        let resolver = LanguageResolver::from_providers(vec![
            into_provider(ContextLanguageProvider),
            into_provider(AcceptLanguageProvider),
            into_provider(DefaultLanguageProvider::new("es")),
        ])
        .unwrap();

        let got = resolver.resolve(&ctx).unwrap();
        assert_eq!(got, "fr");
    }

    #[test]
    fn resolver_provider_error_continues() {
        let resolver = LanguageResolver::from_providers(vec![
            into_provider(StubProvider {
                lang: String::new(),
                fail: true,
            }),
            into_provider(DefaultLanguageProvider::new("en")),
        ])
        .unwrap();

        let got = resolver.resolve(&LanguageContext::new()).unwrap();
        assert_eq!(got, "en");
    }

    #[test]
    fn resolver_no_language() {
        let resolver = LanguageResolver::from_providers(vec![
            into_provider(StubProvider {
                lang: String::new(),
                fail: false,
            }),
            into_provider(StubProvider {
                lang: "   ".to_string(),
                fail: false,
            }),
        ])
        .unwrap();

        let err = resolver.resolve(&LanguageContext::new()).unwrap_err();
        assert_eq!(err, NoLanguageError);
    }

    #[test]
    fn prepend_providers_empty_returns_copy() {
        let base = LanguageResolver::new([DefaultLanguageProvider::new("en")]).unwrap();
        let copy = base
            .prepend_providers(std::iter::empty::<DefaultLanguageProvider>())
            .unwrap();
        assert_eq!(copy.resolve(&LanguageContext::new()).unwrap(), "en");
    }

    #[test]
    fn context_language_provider() {
        let provider = ContextLanguageProvider;
        let ctx = LanguageContext::new().with_language("pt");
        let got = provider.language(&ctx).unwrap().unwrap();
        assert_eq!(got, "pt");
    }

    #[test]
    fn accept_language_provider() {
        let provider = AcceptLanguageProvider;
        let ctx = LanguageContext::new().with_accept_language("en-US,en;q=0.9");
        let got = provider.language(&ctx).unwrap().unwrap();
        assert_eq!(got, "en");
    }

    #[test]
    fn default_language_provider() {
        let provider = DefaultLanguageProvider::new("de");
        let got = provider.language(&LanguageContext::new()).unwrap().unwrap();
        assert_eq!(got, "de");
    }
}
