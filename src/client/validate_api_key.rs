//! `validate_api_key` — GET `/api/v1/api-keys/validate`.

use url::Url;

use crate::models::{ConfigurationError, ValidateApiKeyResponse};

use super::transport::{
    assign_response_context, classify_reqwest_error, decode_json_body, handle_api_error,
    map_transport_failure,
};
use super::transport::{default_headers, endpoint_url, get_method, VALIDATE_API_KEY_PATH};
use super::{Client, Error};
use reqwest::StatusCode;

impl Client {
    /// Validates the configured API key and returns tenant/project scope metadata.
    pub async fn validate_api_key(&self) -> Result<ValidateApiKeyResponse, Error> {
        let raw_url = endpoint_url(&self.base_url, VALIDATE_API_KEY_PATH)?;
        let url = Url::parse(&raw_url).map_err(|err| ConfigurationError {
            message: format!("parse request url: {err}"),
        })?;

        let headers = default_headers(&self.api_key, "application/json", None)?;
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

        if response.status() != StatusCode::OK {
            let status_code = response.status().as_u16();
            let body = response.bytes().await.unwrap_or_default();
            return Err(Error::Api(handle_api_error(status_code, &body)));
        }

        assign_response_context(&response, None, false);
        let body = response
            .bytes()
            .await
            .map_err(|err| map_transport_failure(classify_reqwest_error(&err), self.timeout))?;
        decode_json_body(&body, StatusCode::OK.as_u16()).map_err(Error::Api)
    }
}
