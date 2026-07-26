//! Shared HTTP transport helpers for the live client.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE};
use reqwest::{Method, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::http::build_url;
use crate::models::{
    parse_translaas_error, ApiError, ConfigurationError, ProjectLocales, RequestContext,
    TranslationGroup, TranslationProject,
};

use super::Error;

pub(crate) const SDK_TRANSLATIONS_PREFIX: &str = "sdk/v1/translations";
pub(crate) const VALIDATE_API_KEY_PATH: &str = "api/v1/api-keys/validate";
pub(crate) const HEADER_API_KEY: &str = "X-Api-Key";
pub(crate) const HEADER_IF_NONE_MATCH: &str = "If-None-Match";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransportFailure {
    Timeout,
    Canceled,
    Other(String),
}

pub(crate) fn classify_reqwest_error(err: &reqwest::Error) -> TransportFailure {
    // reqwest 0.12 has no `is_canceled`; detect cancel via message / source chain.
    if error_message_suggests_cancel(err) {
        return TransportFailure::Canceled;
    }
    if err.is_timeout() {
        return TransportFailure::Timeout;
    }
    let message = err.to_string().to_ascii_lowercase();
    if message.contains("timeout") || message.contains("deadline exceeded") {
        return TransportFailure::Timeout;
    }
    TransportFailure::Other(err.to_string())
}

fn error_message_suggests_cancel(err: &reqwest::Error) -> bool {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(err) = current {
        let message = err.to_string().to_ascii_lowercase();
        if message.contains("cancel") {
            return true;
        }
        current = err.source();
    }
    false
}

pub(crate) fn map_transport_failure(failure: TransportFailure, timeout: Duration) -> Error {
    match failure {
        TransportFailure::Canceled => Error::Canceled,
        TransportFailure::Timeout => {
            let seconds = timeout.as_secs_f64();
            Error::Api(ApiError {
                status_code: StatusCode::REQUEST_TIMEOUT.as_u16(),
                code: None,
                message: Some(format!("Request timed out after {seconds} seconds.")),
                response_content: None,
            })
        }
        TransportFailure::Other(message) => Error::Api(ApiError {
            status_code: StatusCode::BAD_REQUEST.as_u16(),
            code: None,
            message: Some(format!("Failed to retrieve translation: {message}")),
            response_content: None,
        }),
    }
}

pub(crate) fn handle_api_error(status_code: u16, body: &[u8]) -> ApiError {
    let content = String::from_utf8_lossy(body).into_owned();
    let fallback = format!("API request failed with status code {status_code}.");

    let mut api_err = ApiError {
        status_code,
        code: None,
        message: Some(fallback.clone()),
        response_content: Some(content),
    };

    if let Ok(Some(parsed)) = parse_translaas_error(body) {
        api_err.message = Some(parsed.format_message(&fallback));
        api_err.code = parsed.code;
    }

    api_err
}

pub(crate) fn assign_response_context(
    response: &Response,
    ctx: Option<&mut RequestContext>,
    not_modified: bool,
) {
    let Some(ctx) = ctx else {
        return;
    };
    ctx.not_modified = not_modified;
    if let Some(etag) = response.headers().get(reqwest::header::ETAG) {
        if let Ok(value) = etag.to_str() {
            if !value.is_empty() {
                ctx.response_etag = Some(value.to_string());
            }
        }
    }
}

pub(crate) fn endpoint_url(base_url: &str, relative: &str) -> Result<String, ConfigurationError> {
    build_url(base_url, relative)
}

pub(crate) fn default_headers(
    api_key: &str,
    accept: &str,
    req_ctx: Option<&RequestContext>,
) -> Result<HeaderMap, Error> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HEADER_API_KEY,
        HeaderValue::from_str(api_key).map_err(|err| ConfigurationError {
            message: format!("invalid API key header value: {err}"),
        })?,
    );
    if !accept.is_empty() {
        headers.insert(
            ACCEPT,
            HeaderValue::from_str(accept).map_err(|err| ConfigurationError {
                message: format!("invalid Accept header value: {err}"),
            })?,
        );
    }
    if let Some(ctx) = req_ctx {
        if let Some(ref if_none_match) = ctx.if_none_match {
            if !if_none_match.is_empty() {
                headers.insert(
                    HEADER_IF_NONE_MATCH,
                    HeaderValue::from_str(if_none_match).map_err(|err| ConfigurationError {
                        message: format!("invalid If-None-Match header value: {err}"),
                    })?,
                );
            }
        }
    }
    Ok(headers)
}

pub(crate) fn get_method() -> Method {
    Method::GET
}

pub(crate) fn post_method() -> Method {
    Method::POST
}

pub(crate) fn require_non_empty(value: &str, name: &str) -> Result<(), ConfigurationError> {
    if value.trim().is_empty() {
        return Err(ConfigurationError {
            message: format!("{name} is required"),
        });
    }
    Ok(())
}

pub(crate) fn apply_channel_version(
    channel: &mut Option<String>,
    version: &mut Option<String>,
    ctx: Option<&RequestContext>,
) {
    let Some(ctx) = ctx else {
        return;
    };
    if let Some(ref value) = ctx.channel {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *channel = Some(trimmed.to_string());
        }
    }
    if let Some(ref value) = ctx.version {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            *version = Some(trimmed.to_string());
        }
    }
}

pub(crate) fn apply_snapshot_context(
    channel: &mut Option<String>,
    version: &mut Option<String>,
    include_context: &mut Option<bool>,
    ctx: Option<&RequestContext>,
) {
    apply_channel_version(channel, version, ctx);
    if let Some(ctx) = ctx {
        if ctx.include_context.is_some() {
            *include_context = ctx.include_context;
        }
    }
}

pub(crate) fn decode_json_body<T: DeserializeOwned>(
    body: &[u8],
    status_code: u16,
) -> Result<T, ApiError> {
    serde_json::from_slice(body).map_err(|err| ApiError {
        status_code,
        code: None,
        message: Some(format!("Failed to decode response body: {err}")),
        response_content: Some(String::from_utf8_lossy(body).into_owned()),
    })
}

pub(crate) fn empty_translation_group() -> TranslationGroup {
    TranslationGroup::default()
}

pub(crate) fn empty_translation_project() -> TranslationProject {
    TranslationProject::default()
}

pub(crate) fn empty_project_locales() -> ProjectLocales {
    ProjectLocales {
        project: None,
        locales: Vec::new(),
        last_modified_utc: None,
    }
}

pub(crate) fn response_etag(response: &Response) -> Option<String> {
    response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// Parses `Content-Disposition` for `filename=` then RFC 5987 `filename*=`.
pub(crate) fn parse_content_disposition(header: &str) -> Option<String> {
    if header.is_empty() {
        return None;
    }

    let lower = header.to_ascii_lowercase();
    if let Some(start) = lower.find("filename=") {
        let value = header[start + "filename=".len()..].trim();
        let value = value.trim_matches('"');
        let value = value.split(';').next().unwrap_or(value).trim();
        if !value.is_empty() && !value.starts_with('*') {
            return Some(value.to_string());
        }
    }

    parse_filename_star(header)
}

fn parse_filename_star(header: &str) -> Option<String> {
    let lower = header.to_ascii_lowercase();
    let prefix = "filename*=";
    let idx = lower.find(prefix)?;
    let mut value = header[idx + prefix.len()..].trim();
    if let Some(semi) = value.find(';') {
        value = value[..semi].trim();
    }
    value = value.trim_matches('"');
    let parts: Vec<&str> = value.splitn(2, "''").collect();
    if parts.len() != 2 {
        return if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
    }
    Some(percent_decode(parts[1]))
}

fn percent_decode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(decoded) =
                u8::from_str_radix(std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or(""), 16)
            {
                out.push(char::from(decoded));
                i += 3;
                continue;
            }
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

pub(crate) fn json_post_headers(api_key: &str) -> Result<HeaderMap, Error> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HEADER_API_KEY,
        HeaderValue::from_str(api_key).map_err(|err| ConfigurationError {
            message: format!("invalid API key header value: {err}"),
        })?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn map_timeout_to_408() {
        let err = map_transport_failure(TransportFailure::Timeout, Duration::from_millis(50));
        let api = err.as_api().expect("api error");
        assert_eq!(api.status_code, 408);
        assert!(api.message.as_deref().unwrap_or("").contains("timed out"));
    }

    #[test]
    fn map_canceled() {
        let err = map_transport_failure(TransportFailure::Canceled, Duration::from_secs(1));
        assert!(err.is_canceled());
    }

    #[test]
    fn handle_api_error_envelope() {
        let body = br#"{"message":"invalid key","code":"AUTH"}"#;
        let err = handle_api_error(401, body);
        assert_eq!(err.status_code, 401);
        assert_eq!(err.message.as_deref(), Some("[AUTH] invalid key"));
        assert_eq!(err.code.as_deref(), Some("AUTH"));
    }

    #[test]
    fn handle_api_error_plain_fallback() {
        let err = handle_api_error(500, b"Internal Server Error");
        assert_eq!(err.status_code, 500);
        assert_eq!(
            err.message.as_deref(),
            Some("API request failed with status code 500.")
        );
        assert_eq!(
            err.response_content.as_deref(),
            Some("Internal Server Error")
        );
    }

    #[test]
    fn parse_content_disposition_filename() {
        let got = parse_content_disposition(r#"attachment; filename="bundle.zip""#);
        assert_eq!(got.as_deref(), Some("bundle.zip"));
    }

    #[test]
    fn parse_content_disposition_filename_star() {
        let got = parse_content_disposition(r#"attachment; filename*=UTF-8''my%20bundle.zip"#);
        assert_eq!(got.as_deref(), Some("my bundle.zip"));
    }

    #[test]
    fn parse_content_disposition_missing() {
        assert_eq!(parse_content_disposition(""), None);
    }

    #[test]
    fn decode_json_body_invalid() {
        let err = decode_json_body::<TranslationGroup>(b"not-json", 200).unwrap_err();
        assert_eq!(err.status_code, 200);
        assert!(err
            .message
            .as_deref()
            .unwrap_or("")
            .contains("Failed to decode response body"));
    }
}
