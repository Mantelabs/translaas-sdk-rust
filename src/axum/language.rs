//! Request language extraction for Axum handlers.

use std::sync::Arc;

use axum::http::request::Parts;

use crate::service::{
    normalize_language_code, parse_accept_language, LanguageContext, LanguageProvider,
    LanguageProviderError,
};

/// Identifies where [`RequestLanguageProvider`] reads a language code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageSource {
    /// Query string parameter (default name `lang`).
    Query,
    /// Raw header value (configured header name).
    Header,
    /// Cookie value (default name `language`).
    Cookie,
    /// Route/path parameter via [`RouteLanguageFn`].
    Route,
    /// Parsed `Accept-Language` header.
    AcceptLanguage,
}

/// Configurable names for request language extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestLanguageOptions {
    /// Query parameter name. Default: `lang`.
    pub query_param: String,
    /// Header name for [`LanguageSource::Header`]. Default: `Accept-Language`.
    pub header_name: String,
    /// Cookie name. Default: `language`.
    pub cookie_name: String,
    /// Route parameter name hint (prefer [`RouteLanguageFn`] for Axum path params).
    pub route_param: String,
}

impl Default for RequestLanguageOptions {
    fn default() -> Self {
        Self {
            query_param: "lang".to_string(),
            header_name: "Accept-Language".to_string(),
            cookie_name: "language".to_string(),
            route_param: String::new(),
        }
    }
}

/// Supplies a route/path parameter language when [`LanguageSource::Route`] is enabled.
pub type RouteLanguageFn = Arc<dyn Fn(&Parts) -> Option<String> + Send + Sync>;

/// Resolves language from HTTP request parts (Go `RequestLanguageProvider` parity).
#[derive(Debug, Clone)]
pub struct RequestLanguageProvider {
    sources: Vec<LanguageSource>,
    query_value: Option<String>,
    header_value: Option<String>,
    cookie_value: Option<String>,
    accept_language_header: Option<String>,
    route_value: Option<String>,
}

impl RequestLanguageProvider {
    /// Builds a provider for the given request parts and source order.
    pub fn from_parts(
        parts: &Parts,
        opts: RequestLanguageOptions,
        sources: Vec<LanguageSource>,
        route_language: Option<RouteLanguageFn>,
    ) -> Self {
        let opts = normalize_request_language_options(opts);
        let accept_language_header = header_value(parts, "Accept-Language");
        let route_value = route_language.and_then(|func| func(parts).map(normalize_language));

        Self {
            query_value: query_value(parts, &opts.query_param).map(normalize_language),
            header_value: header_value(parts, &opts.header_name).map(normalize_language),
            cookie_value: cookie_value(parts, &opts.cookie_name).map(normalize_language),
            accept_language_header,
            route_value,
            sources,
        }
    }

    fn language_from_source(&self, source: LanguageSource) -> Option<String> {
        match source {
            LanguageSource::Query => self.query_value.clone(),
            LanguageSource::Header => self.header_value.clone(),
            LanguageSource::Cookie => self.cookie_value.clone(),
            LanguageSource::Route => self.route_value.clone(),
            LanguageSource::AcceptLanguage => self
                .accept_language_header
                .as_deref()
                .map(parse_accept_language)
                .filter(|lang| !lang.is_empty()),
        }
    }
}

impl LanguageProvider for RequestLanguageProvider {
    fn language(&self, _ctx: &LanguageContext) -> Result<Option<String>, LanguageProviderError> {
        for source in &self.sources {
            if let Some(lang) = self.language_from_source(*source) {
                return Ok(Some(lang));
            }
        }
        Ok(None)
    }
}

/// Builds [`LanguageContext`] from request parts and optional resolved language.
pub fn language_context_from_parts(
    parts: &Parts,
    resolved_language: Option<String>,
) -> LanguageContext {
    let mut ctx = LanguageContext::new();
    if let Some(header) = header_value(parts, "Accept-Language") {
        ctx = ctx.with_accept_language(header);
    }
    if let Some(lang) = resolved_language.filter(|value| !value.is_empty()) {
        ctx = ctx.with_language(lang);
    }
    ctx
}

/// Default language source order (Go parity): query, Accept-Language, cookie.
pub fn default_language_sources() -> Vec<LanguageSource> {
    vec![
        LanguageSource::Query,
        LanguageSource::AcceptLanguage,
        LanguageSource::Cookie,
    ]
}

fn normalize_request_language_options(opts: RequestLanguageOptions) -> RequestLanguageOptions {
    let mut opts = opts;
    if opts.query_param.is_empty() {
        opts.query_param = "lang".to_string();
    }
    if opts.header_name.is_empty() {
        opts.header_name = "Accept-Language".to_string();
    }
    if opts.cookie_name.is_empty() {
        opts.cookie_name = "language".to_string();
    }
    opts
}

fn normalize_language(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_language_code(trimmed);
    if normalized.is_empty() {
        trimmed.to_ascii_lowercase()
    } else {
        normalized
    }
}

fn query_value(parts: &Parts, param: &str) -> Option<String> {
    let query = parts.uri.query()?;
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == param)
        .map(|(_, value)| value.into_owned())
        .filter(|value| !value.trim().is_empty())
}

fn header_value(parts: &Parts, name: &str) -> Option<String> {
    parts
        .headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
        .filter(|value| !value.trim().is_empty())
}

fn cookie_value(parts: &Parts, name: &str) -> Option<String> {
    let header = parts.headers.get(axum::http::header::COOKIE)?;
    let header = header.to_str().ok()?;
    parse_cookie(header, name)
}

fn parse_cookie(header: &str, name: &str) -> Option<String> {
    for part in header.split(';') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            if key.trim() == name {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    fn parts_from_request(request: Request<()>) -> Parts {
        request.into_parts().0
    }

    #[test]
    fn request_language_source_order_query_wins() {
        let mut request = Request::builder().uri("/?lang=de").body(()).unwrap();
        request
            .headers_mut()
            .insert("Accept-Language", "fr-FR".parse().unwrap());
        request
            .headers_mut()
            .insert("Cookie", "language=es; other=1".parse().unwrap());

        let parts = parts_from_request(request);
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions::default(),
            vec![
                LanguageSource::Query,
                LanguageSource::AcceptLanguage,
                LanguageSource::Cookie,
            ],
            None,
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert_eq!(lang.as_deref(), Some("de"));
    }

    #[test]
    fn request_language_accept_language_parsing() {
        let request = Request::builder()
            .uri("/")
            .header("Accept-Language", "en-US,en;q=0.9")
            .body(())
            .unwrap();
        let parts = parts_from_request(request);
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions::default(),
            vec![LanguageSource::AcceptLanguage],
            None,
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert_eq!(lang.as_deref(), Some("en"));
    }

    #[test]
    fn request_language_cookie_source() {
        let request = Request::builder()
            .uri("/")
            .header("Cookie", "language=es")
            .body(())
            .unwrap();
        let parts = parts_from_request(request);
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions::default(),
            vec![LanguageSource::Cookie],
            None,
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert_eq!(lang.as_deref(), Some("es"));
    }

    #[test]
    fn request_language_route_func() {
        let request = Request::builder().uri("/").body(()).unwrap();
        let parts = parts_from_request(request);
        let route = Arc::new(|_: &Parts| Some("pt".to_string()));
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions::default(),
            vec![LanguageSource::Route],
            Some(route),
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert_eq!(lang.as_deref(), Some("pt"));
    }

    #[test]
    fn request_language_header_source() {
        let request = Request::builder()
            .uri("/")
            .header("X-Lang", "ja")
            .body(())
            .unwrap();
        let parts = parts_from_request(request);
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions {
                header_name: "X-Lang".to_string(),
                ..RequestLanguageOptions::default()
            },
            vec![LanguageSource::Header],
            None,
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert_eq!(lang.as_deref(), Some("ja"));
    }

    #[test]
    fn request_language_options_defaults() {
        let opts = normalize_request_language_options(RequestLanguageOptions {
            query_param: String::new(),
            header_name: String::new(),
            cookie_name: String::new(),
            route_param: String::new(),
        });

        assert_eq!(opts.query_param, "lang");
        assert_eq!(opts.header_name, "Accept-Language");
        assert_eq!(opts.cookie_name, "language");
    }

    #[test]
    fn request_language_empty_sources_skip() {
        let request = Request::builder().uri("/").body(()).unwrap();
        let parts = parts_from_request(request);
        let provider = RequestLanguageProvider::from_parts(
            &parts,
            RequestLanguageOptions::default(),
            vec![LanguageSource::Query, LanguageSource::Cookie],
            None,
        );

        let lang = provider.language(&LanguageContext::new()).unwrap();
        assert!(lang.is_none());
    }

    #[test]
    fn language_context_from_parts_sets_accept_language_and_resolved_lang() {
        let request = Request::builder()
            .uri("/?lang=de")
            .header("Accept-Language", "fr-FR")
            .body(())
            .unwrap();
        let parts = parts_from_request(request);

        let ctx = language_context_from_parts(&parts, Some("de".to_string()));
        assert_eq!(ctx.accept_language(), Some("fr-FR"));
        assert_eq!(ctx.language(), Some("de"));
    }
}
