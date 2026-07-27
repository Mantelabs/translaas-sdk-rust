//! `get_entry` — GET `/sdk/v1/translations/text`.

use std::collections::HashMap;

use reqwest::StatusCode;
use url::Url;

use crate::http::{append_query_values, inject_plural_n, merge_query_params};
use crate::models::{ConfigurationError, GetTranslationRequest, RequestContext};

#[cfg(feature = "cache")]
use super::cache_integration::build_entry_cache_key;

use super::transport::{
    assign_response_context, classify_reqwest_error, default_headers, endpoint_url, get_method,
    handle_api_error, map_transport_failure, SDK_TRANSLATIONS_PREFIX,
};
use super::{Client, Error};

/// Per-call options for [`Client::get_entry`].
#[derive(Debug, Default)]
pub struct GetEntryOptions<'a> {
    /// Plural / interpolation count (`n` / `N` query parameters).
    pub number: Option<f64>,
    /// Extra interpolation query parameters.
    pub parameters: HashMap<String, String>,
    /// Per-request channel, version, project, and conditional headers.
    pub request_context: Option<&'a mut RequestContext>,
}

impl<'a> GetEntryOptions<'a> {
    /// Creates empty options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the plural number.
    pub fn number(mut self, number: f64) -> Self {
        self.number = Some(number);
        self
    }

    /// Sets interpolation parameters.
    pub fn parameters(mut self, parameters: HashMap<String, String>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Sets the mutable request context.
    pub fn request_context(mut self, request_context: &'a mut RequestContext) -> Self {
        self.request_context = Some(request_context);
        self
    }
}

impl Client {
    /// Retrieves a single rendered translation string (plain text body).
    ///
    /// Calls `GET /sdk/v1/translations/text`. Status handling:
    /// - **200** → response body text
    /// - **204** → returns `entry` unchanged
    /// - **304** → empty string and sets `request_context.not_modified`
    pub async fn get_entry(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        mut opts: GetEntryOptions<'_>,
    ) -> Result<String, Error> {
        require_non_empty(group, "group")?;
        require_non_empty(entry, "entry")?;
        require_non_empty(lang, "lang")?;

        if let Some(ref mut ctx) = opts.request_context {
            ctx.reset();
        }

        #[cfg(feature = "cache")]
        let cache_key = if self.caching_enabled("entry") {
            Some(build_entry_cache_key(
                group,
                entry,
                lang,
                opts.number,
                &opts.parameters,
                opts.request_context.as_deref(),
                self.default_project_id.as_deref(),
            ))
        } else {
            None
        };

        #[cfg(feature = "cache")]
        if let Some(ref key) = cache_key {
            if let Some(cached) = self.try_cache_get_string(key) {
                return Ok(cached);
            }
        }

        let project = resolve_entry_project(
            opts.request_context.as_deref(),
            self.default_project_id.as_deref(),
        );
        let channel = opts
            .request_context
            .as_ref()
            .and_then(|ctx| ctx.channel.clone());
        let version = opts
            .request_context
            .as_ref()
            .and_then(|ctx| ctx.version.clone());

        let req_model = GetTranslationRequest {
            group: Some(group.to_string()),
            entry: Some(entry.to_string()),
            lang: Some(lang.to_string()),
            n: opts.number,
            project,
            channel,
            version,
        };

        let mut extra = opts.parameters.clone();
        inject_plural_n(&mut extra, opts.number);

        let raw_url = endpoint_url(&self.base_url, &format!("{SDK_TRANSLATIONS_PREFIX}/text"))?;
        let mut url = Url::parse(&raw_url).map_err(|err| ConfigurationError {
            message: format!("parse request url: {err}"),
        })?;
        append_query_values(&mut url, &req_model)?;
        merge_query_params(&mut url, &extra);

        let headers =
            default_headers(&self.api_key, "text/plain", opts.request_context.as_deref())?;

        let request = self.http_client.request(get_method(), url).headers(headers);

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                return Err(map_transport_failure(
                    classify_reqwest_error(&err),
                    self.timeout,
                ));
            }
        };

        match response.status() {
            StatusCode::OK => {
                assign_response_context(&response, opts.request_context.as_deref_mut(), false);
                let body = response.text().await.map_err(|err| {
                    map_transport_failure(classify_reqwest_error(&err), self.timeout)
                })?;
                #[cfg(feature = "cache")]
                if let Some(ref key) = cache_key {
                    self.cache_set_string(key, &body);
                }
                Ok(body)
            }
            StatusCode::NO_CONTENT => {
                assign_response_context(&response, opts.request_context.as_deref_mut(), false);
                Ok(entry.to_string())
            }
            StatusCode::NOT_MODIFIED => {
                assign_response_context(&response, opts.request_context.as_deref_mut(), true);
                #[cfg(feature = "cache")]
                if self.has_cache_provider() {
                    if let Some(ref key) = cache_key {
                        if let Some(cached) = self.try_cache_get_string(key) {
                            return Ok(cached);
                        }
                    }
                }
                Ok(String::new())
            }
            status => {
                let status_code = status.as_u16();
                let body = response.bytes().await.unwrap_or_default();
                Err(Error::Api(handle_api_error(status_code, &body)))
            }
        }
    }
}

fn require_non_empty(value: &str, name: &str) -> Result<(), ConfigurationError> {
    if value.trim().is_empty() {
        return Err(ConfigurationError {
            message: format!("{name} is required"),
        });
    }
    Ok(())
}

fn resolve_entry_project(
    ctx: Option<&RequestContext>,
    default_project_id: Option<&str>,
) -> Option<String> {
    #[cfg(feature = "cache")]
    {
        let project = super::cache_integration::resolve_entry_project(ctx, default_project_id);
        if project.is_empty() {
            None
        } else {
            Some(project.to_string())
        }
    }
    #[cfg(not(feature = "cache"))]
    {
        if let Some(ctx) = ctx {
            if let Some(ref project) = ctx.project {
                let trimmed = project.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_string());
                }
            }
        }
        default_project_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
    }
}
