//! `report_missing_keys` — POST `/sdk/v1/translations/report-missing`.

use reqwest::StatusCode;

use crate::http::build_url;
use crate::models::{ConfigurationError, ReportMissingKeyItem, ReportMissingKeysRequest};

use super::transport::{
    classify_reqwest_error, handle_api_error, json_post_headers, map_transport_failure,
    post_method, SDK_TRANSLATIONS_PREFIX,
};
use super::{Client, Error};

impl Client {
    /// Reports translation keys that could not be resolved at runtime.
    ///
    /// Returns immediately without a network call when `keys` is empty.
    pub async fn report_missing_keys(&self, keys: &[ReportMissingKeyItem]) -> Result<(), Error> {
        if keys.is_empty() {
            return Ok(());
        }

        let raw_url = build_url(
            &self.base_url,
            &format!("{SDK_TRANSLATIONS_PREFIX}/report-missing"),
        )?;
        let url = reqwest::Url::parse(&raw_url).map_err(|err| ConfigurationError {
            message: format!("parse request url: {err}"),
        })?;

        let body = ReportMissingKeysRequest {
            keys: keys.to_vec(),
        };
        let headers = json_post_headers(&self.api_key)?;
        let request = self
            .http_client
            .request(post_method(), url)
            .headers(headers)
            .json(&body);

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                return Err(map_transport_failure(
                    classify_reqwest_error(&err),
                    self.timeout,
                ));
            }
        };

        if response.status() == StatusCode::ACCEPTED {
            return Ok(());
        }

        let status_code = response.status().as_u16();
        let body = response.bytes().await.unwrap_or_default();
        Err(Error::Api(handle_api_error(status_code, &body)))
    }
}
