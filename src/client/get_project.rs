//! `get_project` — GET `/sdk/v1/translations/project`.

use crate::models::{GetProjectTranslationsRequest, RequestContext, TranslationProject};

use super::json_get::execute_json_get;
use super::transport::{apply_snapshot_context, empty_translation_project, require_non_empty};
use super::{Client, Error};

/// Per-call options for [`Client::get_project`].
#[derive(Debug, Default)]
pub struct GetProjectOptions<'a> {
    /// Response format query parameter (for example `flat-json`).
    pub format: Option<String>,
    /// Per-request channel, version, includeContext, and conditional headers.
    pub request_context: Option<&'a mut RequestContext>,
}

impl<'a> GetProjectOptions<'a> {
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
    /// Retrieves all translation groups for a project and language.
    pub async fn get_project(
        &self,
        project: &str,
        lang: &str,
        mut opts: GetProjectOptions<'_>,
    ) -> Result<TranslationProject, Error> {
        require_non_empty(project, "project")?;
        require_non_empty(lang, "lang")?;

        if let Some(ref mut ctx) = opts.request_context {
            ctx.reset();
        }

        let mut req_model = GetProjectTranslationsRequest {
            project: Some(project.to_string()),
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

        execute_json_get(
            self,
            "project",
            &req_model,
            opts.request_context.as_deref_mut(),
            empty_translation_project,
        )
        .await
    }
}
