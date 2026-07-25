//! Base URL and endpoint joining.

use crate::models::ConfigurationError;
use url::Url;

/// Joins `base_url` and `endpoint` like .NET `TranslaasClient.BuildEndpointUrl`.
///
/// Trims trailing slashes from the base and leading slashes from the endpoint.
/// Returns an error when the base URL is empty, not parseable, or lacks an
/// `http`/`https` scheme with a host.
pub(crate) fn build_url(base_url: &str, endpoint: &str) -> Result<String, ConfigurationError> {
    let base = base_url.trim();
    if base.is_empty() {
        return Err(ConfigurationError {
            message: "http: base URL is required".to_string(),
        });
    }

    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err(ConfigurationError {
            message: "http: endpoint is required".to_string(),
        });
    }

    let parsed = Url::parse(base).map_err(|err| ConfigurationError {
        message: format!("http: parse base URL: {err}"),
    })?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(ConfigurationError {
                message: "http: base URL must use http or https scheme".to_string(),
            });
        }
    }

    if parsed.host_str().unwrap_or("").is_empty() {
        return Err(ConfigurationError {
            message: "http: base URL must include a host".to_string(),
        });
    }

    let trimmed_base = base.trim_end_matches('/');
    let endpoint_path = endpoint.trim_start_matches('/');
    Ok(format!("{trimmed_base}/{endpoint_path}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_combines_base_and_endpoint() {
        let got = build_url("https://api.test.com", "sdk/v1/translations/text").unwrap();
        assert_eq!(got, "https://api.test.com/sdk/v1/translations/text");
    }

    #[test]
    fn build_url_trims_slashes() {
        let got = build_url("https://api.test.com/", "/sdk/v1/translations/text").unwrap();
        assert_eq!(got, "https://api.test.com/sdk/v1/translations/text");
    }

    #[test]
    fn build_url_preserves_base_path_prefix() {
        let got = build_url("https://api.test.com/custom", "sdk/v1/translations/text").unwrap();
        assert_eq!(got, "https://api.test.com/custom/sdk/v1/translations/text");
    }

    #[test]
    fn build_url_validate_api_key_path() {
        let got = build_url("https://api.test.com", "api/v1/api-keys/validate").unwrap();
        assert_eq!(got, "https://api.test.com/api/v1/api-keys/validate");
    }

    #[test]
    fn build_url_trims_whitespace_base() {
        let got = build_url("  https://api.test.com  ", "sdk/v1/translations/text").unwrap();
        assert_eq!(got, "https://api.test.com/sdk/v1/translations/text");
    }

    #[test]
    fn build_url_rejects_invalid_base() {
        assert!(build_url("   ", "sdk/v1/translations/text").is_err());
        assert!(build_url("not-a-url", "sdk/v1/translations/text").is_err());
        assert!(build_url("ftp://files.test.com", "sdk/v1/translations/text").is_err());
        assert!(build_url("https://api.test.com", "   ").is_err());
    }
}
