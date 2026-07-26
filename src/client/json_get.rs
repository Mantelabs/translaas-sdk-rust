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
) -> Result<T, Error>
where
    T: DeserializeOwned,
    F: FnOnce() -> T,
{
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

    handle_json_response(response, ctx, empty, client.timeout).await
}

async fn handle_json_response<T, F>(
    response: reqwest::Response,
    ctx: Option<&mut RequestContext>,
    empty: F,
    timeout: std::time::Duration,
) -> Result<T, Error>
where
    T: DeserializeOwned,
    F: FnOnce() -> T,
{
    match response.status() {
        StatusCode::OK => {
            assign_response_context(&response, ctx, false);
            let status_code = response.status().as_u16();
            let body = response
                .bytes()
                .await
                .map_err(|err| map_transport_failure(classify_reqwest_error(&err), timeout))?;
            decode_json_body(&body, status_code).map_err(Error::Api)
        }
        StatusCode::NO_CONTENT => {
            assign_response_context(&response, ctx, false);
            Ok(empty())
        }
        StatusCode::NOT_MODIFIED => {
            assign_response_context(&response, ctx, true);
            Ok(empty())
        }
        status => {
            let status_code = status.as_u16();
            let body = response.bytes().await.unwrap_or_default();
            Err(Error::Api(handle_api_error(status_code, &body)))
        }
    }
}
