//! Classifies client errors eligible for offline cache fallback.

use crate::client::Error;

/// Returns true when a failed API call may fall back to the offline cache.
///
/// Matches Go `cachefile.isNetworkOrAPIError`: API and transport failures qualify;
/// user cancellation does not.
pub(crate) fn is_network_or_api_error(err: &Error) -> bool {
    match err {
        Error::Canceled => false,
        Error::Api(_) => true,
        Error::Configuration(_) | Error::OfflineCache(_) | Error::OfflineCacheMiss(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ApiError;

    #[test]
    fn canceled_is_not_eligible() {
        assert!(!is_network_or_api_error(&Error::Canceled));
    }

    #[test]
    fn api_error_is_eligible() {
        let err = Error::Api(ApiError {
            status_code: 502,
            code: None,
            message: Some("bad gateway".to_string()),
            response_content: None,
        });
        assert!(is_network_or_api_error(&err));
    }

    #[test]
    fn configuration_error_is_not_eligible() {
        let err = Error::Configuration(crate::models::ConfigurationError {
            message: "invalid".to_string(),
        });
        assert!(!is_network_or_api_error(&err));
    }
}
