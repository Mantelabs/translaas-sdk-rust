//! `get_project_locales` — GET `/sdk/v1/translations/locales`.

use crate::models::{GetProjectLocalesRequest, ProjectLocales, RequestContext};

#[cfg(feature = "cache")]
use crate::cache::KeyBuilder;

use super::json_get::execute_json_get;
use super::transport::{apply_channel_version, empty_project_locales, require_non_empty};
use super::{Client, Error};

/// Per-call options for [`Client::get_project_locales`].
#[derive(Debug, Default)]
pub struct GetProjectLocalesOptions<'a> {
    /// Per-request channel, version, and conditional headers.
    pub request_context: Option<&'a mut RequestContext>,
}

impl<'a> GetProjectLocalesOptions<'a> {
    /// Creates empty options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the mutable request context.
    pub fn request_context(mut self, request_context: &'a mut RequestContext) -> Self {
        self.request_context = Some(request_context);
        self
    }
}

impl Client {
    /// Lists locales available for a project.
    pub async fn get_project_locales(
        &self,
        project: &str,
        mut opts: GetProjectLocalesOptions<'_>,
    ) -> Result<ProjectLocales, Error> {
        require_non_empty(project, "project")?;

        if let Some(ref mut ctx) = opts.request_context {
            ctx.reset();
        }

        let mut req_model = GetProjectLocalesRequest {
            project: Some(project.to_string()),
            channel: None,
            version: None,
        };
        apply_channel_version(
            &mut req_model.channel,
            &mut req_model.version,
            opts.request_context.as_deref(),
        );

        let (cache_op, cache_key) = locales_cache_context(self, project, &req_model);

        execute_json_get(
            self,
            "locales",
            &req_model,
            opts.request_context.as_deref_mut(),
            empty_project_locales,
            cache_op,
            cache_key.as_deref(),
        )
        .await
    }
}

#[cfg(feature = "cache")]
fn locales_cache_context(
    client: &Client,
    project: &str,
    req_model: &GetProjectLocalesRequest,
) -> (Option<&'static str>, Option<String>) {
    if !client.caching_enabled("locales") {
        return (None, None);
    }
    let key = KeyBuilder.locales_key(
        project,
        req_model.channel.as_deref().unwrap_or(""),
        req_model.version.as_deref().unwrap_or(""),
    );
    (Some("locales"), Some(key))
}

#[cfg(not(feature = "cache"))]
fn locales_cache_context(
    _client: &Client,
    _project: &str,
    _req_model: &GetProjectLocalesRequest,
) -> (Option<&'static str>, Option<String>) {
    (None, None)
}
