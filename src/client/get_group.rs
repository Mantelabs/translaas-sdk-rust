//! `get_group` — GET `/sdk/v1/translations/group`.

use crate::models::{GetGroupTranslationsRequest, RequestContext, TranslationGroup};

#[cfg(feature = "cache")]
use crate::cache::KeyBuilder;

use super::json_get::execute_json_get;
use super::transport::{apply_snapshot_context, empty_translation_group, require_non_empty};
use super::{Client, Error};

/// Per-call options for [`Client::get_group`].
#[derive(Debug, Default)]
pub struct GetGroupOptions<'a> {
    /// Response format query parameter (for example `flat-json`).
    pub format: Option<String>,
    /// Per-request channel, version, includeContext, and conditional headers.
    pub request_context: Option<&'a mut RequestContext>,
}

impl<'a> GetGroupOptions<'a> {
    /// Creates empty options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the response format.
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Sets the mutable request context.
    pub fn request_context(mut self, request_context: &'a mut RequestContext) -> Self {
        self.request_context = Some(request_context);
        self
    }
}

impl Client {
    /// Retrieves one translation group for a project and language.
    pub async fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        mut opts: GetGroupOptions<'_>,
    ) -> Result<TranslationGroup, Error> {
        require_non_empty(project, "project")?;
        require_non_empty(group, "group")?;
        require_non_empty(lang, "lang")?;

        if let Some(ref mut ctx) = opts.request_context {
            ctx.reset();
        }

        let mut req_model = GetGroupTranslationsRequest {
            project: Some(project.to_string()),
            group: Some(group.to_string()),
            lang: Some(lang.to_string()),
            format: opts.format.clone(),
            channel: None,
            version: None,
            include_context: None,
        };
        apply_snapshot_context(
            &mut req_model.channel,
            &mut req_model.version,
            &mut req_model.include_context,
            opts.request_context.as_deref(),
        );

        let (cache_op, cache_key) =
            group_cache_context(self, project, group, lang, &opts, &req_model);

        execute_json_get(
            self,
            "group",
            &req_model,
            opts.request_context.as_deref_mut(),
            empty_translation_group,
            cache_op,
            cache_key.as_deref(),
        )
        .await
    }
}

#[cfg(feature = "cache")]
fn group_cache_context(
    client: &Client,
    project: &str,
    group: &str,
    lang: &str,
    opts: &GetGroupOptions<'_>,
    req_model: &GetGroupTranslationsRequest,
) -> (Option<&'static str>, Option<String>) {
    if !client.caching_enabled("group") {
        return (None, None);
    }
    let key = KeyBuilder.group_key(
        project,
        group,
        lang,
        opts.format.as_deref().unwrap_or(""),
        req_model.channel.as_deref().unwrap_or(""),
        req_model.version.as_deref().unwrap_or(""),
        req_model.include_context,
    );
    (Some("group"), Some(key))
}

#[cfg(not(feature = "cache"))]
fn group_cache_context(
    _client: &Client,
    _project: &str,
    _group: &str,
    _lang: &str,
    _opts: &GetGroupOptions<'_>,
    _req_model: &GetGroupTranslationsRequest,
) -> (Option<&'static str>, Option<String>) {
    (None, None)
}
