//! Options validation shared across modules (internal).

use std::time::Duration;

use crate::models::ConfigurationError;

/// Fields validated when constructing a live HTTP client.
#[derive(Debug, Clone)]
pub(crate) struct ClientOptions<'a> {
    pub api_key: &'a str,
    pub base_url: &'a str,
    pub timeout: Option<Duration>,
}

/// Validates live client configuration, mirroring Go `internal/validate.Client`
/// / .NET `TranslaasClientOptions.Validate`.
pub(crate) fn client(opts: ClientOptions<'_>) -> Result<(), ConfigurationError> {
    if opts.api_key.trim().is_empty() {
        return Err(ConfigurationError {
            message: "ApiKey is required and cannot be null or empty.".to_string(),
        });
    }

    let base_url = opts.base_url.trim();
    if base_url.is_empty() {
        return Err(ConfigurationError {
            message: "BaseUrl is required and cannot be null or empty.".to_string(),
        });
    }

    let lower = base_url.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(ConfigurationError {
            message: format!(
                "BaseUrl must be a valid HTTP or HTTPS URL. Provided value: {}",
                opts.base_url
            ),
        });
    }

    if let Some(timeout) = opts.timeout {
        if timeout.is_zero() {
            // Zero means "use default" at the builder layer; treat as valid here.
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn rejects_empty_api_key() {
        let err = client(ClientOptions {
            api_key: "  ",
            base_url: "https://api.test.com",
            timeout: None,
        })
        .unwrap_err();
        assert!(err.message.contains("ApiKey"));
    }

    #[test]
    fn rejects_empty_base_url() {
        let err = client(ClientOptions {
            api_key: "key",
            base_url: "",
            timeout: None,
        })
        .unwrap_err();
        assert!(err.message.contains("BaseUrl"));
    }

    #[test]
    fn rejects_non_http_base_url() {
        let err = client(ClientOptions {
            api_key: "key",
            base_url: "ftp://api.test.com",
            timeout: None,
        })
        .unwrap_err();
        assert!(err.message.contains("HTTP or HTTPS"));
    }

    #[test]
    fn accepts_zero_timeout() {
        client(ClientOptions {
            api_key: "key",
            base_url: "https://api.test.com",
            timeout: Some(Duration::ZERO),
        })
        .unwrap();
    }

    #[test]
    fn accepts_valid_options() {
        client(ClientOptions {
            api_key: "key",
            base_url: "https://api.test.com",
            timeout: Some(Duration::from_secs(5)),
        })
        .unwrap();
    }
}
