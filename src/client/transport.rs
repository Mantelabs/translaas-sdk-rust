//! Shared HTTP transport helpers for the live client.

use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, ACCEPT};
use reqwest::{Method, Response, StatusCode};

use crate::http::build_url;
use crate::models::{parse_translaas_error, ApiError, ConfigurationError, RequestContext};

use super::Error;

pub(crate) const SDK_TRANSLATIONS_PREFIX: &str = "sdk/v1/translations";
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
}
