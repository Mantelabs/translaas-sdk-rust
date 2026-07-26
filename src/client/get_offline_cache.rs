//! `get_offline_cache` — GET `/sdk/v1/translations/offline-cache`.

use reqwest::StatusCode;
use url::Url;

use crate::http::append_query_values;
use crate::models::{
    ConfigurationError, GetOfflineCacheRequest, OfflineCacheDownloadResult, RequestContext,
};

use super::transport::{
    apply_snapshot_context, assign_response_context, classify_reqwest_error, default_headers,
    endpoint_url, get_method, handle_api_error, map_transport_failure, parse_content_disposition,
    require_non_empty, response_etag, SDK_TRANSLATIONS_PREFIX,
};
use super::{Client, Error};

/// Per-call options for [`Client::get_offline_cache`].
#[derive(Debug, Default)]
pub struct GetOfflineCacheOptions<'a> {
    /// Per-request channel, version, includeContext, and conditional headers.
    pub request_context: Option<&'a mut RequestContext>,
}

impl<'a> GetOfflineCacheOptions<'a> {
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
    /// Downloads the offline translation bundle as a ZIP archive.
    pub async fn get_offline_cache(
        &self,
        project: &str,
        mut opts: GetOfflineCacheOptions<'_>,
    ) -> Result<OfflineCacheDownloadResult, Error> {
        require_non_empty(project, "project")?;

        if let Some(ref mut ctx) = opts.request_context {
            ctx.reset();
        }

        let mut req_model = GetOfflineCacheRequest {
            project: Some(project.to_string()),
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

        let raw_url = endpoint_url(
            &self.base_url,
            &format!("{SDK_TRANSLATIONS_PREFIX}/offline-cache"),
        )?;
        let mut url = Url::parse(&raw_url).map_err(|err| ConfigurationError {
            message: format!("parse request url: {err}"),
        })?;
        append_query_values(&mut url, &req_model)?;

        let headers = default_headers(
            &self.api_key,
            "application/zip",
            opts.request_context.as_deref(),
        )?;
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

        let mut result = OfflineCacheDownloadResult::default();
        match response.status() {
            StatusCode::OK => {
                assign_response_context(&response, opts.request_context.as_deref_mut(), false);
                result.etag = response_etag(&response);
                if let Some(disposition) = response
                    .headers()
                    .get(reqwest::header::CONTENT_DISPOSITION)
                    .and_then(|value| value.to_str().ok())
                {
                    result.suggested_file_name = parse_content_disposition(disposition);
                }
                let body = response.bytes().await.map_err(|err| {
                    map_transport_failure(classify_reqwest_error(&err), self.timeout)
                })?;
                result.content = Some(body.to_vec());
                Ok(result)
            }
            StatusCode::NOT_MODIFIED => {
                assign_response_context(&response, opts.request_context.as_deref_mut(), true);
                result.not_modified = true;
                result.etag = response_etag(&response);
                Ok(result)
            }
            status => {
                let status_code = status.as_u16();
                let body = response.bytes().await.unwrap_or_default();
                Err(Error::Api(handle_api_error(status_code, &body)))
            }
        }
    }
}
