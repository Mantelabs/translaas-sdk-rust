//! Shared JSON GET execution for group, project, and locales endpoints.

use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use url::Url;

use crate::http::append_query_values;
use crate::models::{ConfigurationError, RequestContext};

use super::transport::{
    assign_response_context, classify_reqwest_error, decode_json_body, default_headers,
    endpoint_url, get_method, handle_api_error, map_transport_failure, SDK_TRANSLATIONS_PREFIX,
};
use super::{Client, Error};

pub(crate) async fn execute_json_get<T, F>(
    client: &Client,
    path_suffix: &str,
    query_model: &impl Serialize,
    ctx: Option<&mut RequestContext>,
    empty: F,
    cache_op: Option<&str>,
    cache_key: Option<&str>,
) -> Result<T, Error>
where
    T: DeserializeOwned + Clone + Send + Sync + 'static,
    F: FnOnce() -> T,
{
    #[cfg(feature = "cache")]
    if let (Some(op), Some(key)) = (cache_op, cache_key) {
        if client.caching_enabled(op) {
            if let Some(cached) = client.try_cache_get::<T>(key) {
                return Ok(cached);
            }
        }
    }

    let raw_url = endpoint_url(
        &client.base_url,
        &format!("{SDK_TRANSLATIONS_PREFIX}/{path_suffix}"),
    )?;
    let mut url = Url::parse(&raw_url).map_err(|err| ConfigurationError {
        message: format!("parse request url: {err}"),
    })?;
    append_query_values(&mut url, query_model)?;

    let headers = default_headers(&client.api_key, "application/json", ctx.as_deref())?;
    let request = client
        .http_client
        .request(get_method(), url)
        .headers(headers);

    let response = match request.send().await {
        Ok(response) => response,
        Err(err) => {
            return Err(map_transport_failure(
                classify_reqwest_error(&err),
                client.timeout,
            ));
        }
    };

    handle_json_response(response, ctx, empty, client, cache_op, cache_key).await
}

async fn handle_json_response<T, F>(
    response: reqwest::Response,
    ctx: Option<&mut RequestContext>,
    empty: F,
    client: &Client,
    #[cfg_attr(not(feature = "cache"), allow(unused_variables))] cache_op: Option<&str>,
    #[cfg_attr(not(feature = "cache"), allow(unused_variables))] cache_key: Option<&str>,
) -> Result<T, Error>
where
    T: DeserializeOwned + Clone + Send + Sync + 'static,
    F: FnOnce() -> T,
{
    match response.status() {
        StatusCode::OK => {
            assign_response_context(&response, ctx, false);
            let status_code = response.status().as_u16();
            let body = response.bytes().await.map_err(|err| {
                map_transport_failure(classify_reqwest_error(&err), client.timeout)
            })?;
            let decoded: T = decode_json_body(&body, status_code).map_err(Error::Api)?;
            #[cfg(feature = "cache")]
            if let (Some(op), Some(key)) = (cache_op, cache_key) {
                if client.caching_enabled(op) {
                    client.cache_set(key, decoded.clone());
                }
            }
            Ok(decoded)
        }
        StatusCode::NO_CONTENT => {
            assign_response_context(&response, ctx, false);
            Ok(empty())
        }
        StatusCode::NOT_MODIFIED => {
            assign_response_context(&response, ctx, true);
            #[cfg(feature = "cache")]
            if client.has_cache_provider() {
                if let Some(key) = cache_key {
                    if let Some(cached) = client.try_cache_get::<T>(key) {
                        return Ok(cached);
                    }
                }
            }
            Ok(empty())
        }
        status => {
            let status_code = status.as_u16();
            let body = response.bytes().await.unwrap_or_default();
            Err(Error::Api(handle_api_error(status_code, &body)))
        }
    }
}
