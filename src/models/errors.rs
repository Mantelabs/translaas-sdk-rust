//! Typed errors and API error envelope parsing.

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// JSON error envelope returned by the Translaas API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranslaasError {
    /// Human-readable error message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Machine-readable error code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl TranslaasError {
    /// Returns a display message matching .NET client formatting:
    /// `"[code] message"` when code is present, otherwise message or fallback.
    pub fn format_message(&self, fallback: &str) -> String {
        let msg = self.message.as_deref().unwrap_or(fallback);
        if let Some(ref code) = self.code {
            if !code.is_empty() {
                return format!("[{code}] {msg}");
            }
        }
        msg.to_string()
    }
}

/// Unmarshals an API error body. Returns `Ok(None)` when body is empty.
pub fn parse_translaas_error(body: &[u8]) -> Result<Option<TranslaasError>, serde_json::Error> {
    if body.is_empty() {
        return Ok(None);
    }
    let err: TranslaasError = serde_json::from_slice(body)?;
    Ok(Some(err))
}

/// HTTP failure from the Translaas API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// HTTP status code.
    pub status_code: u16,
    /// API error code when present.
    pub code: Option<String>,
    /// Error message when present.
    pub message: Option<String>,
    /// Raw response body for diagnostics.
    pub response_content: Option<String>,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref msg) = self.message {
            if !msg.is_empty() {
                return write!(f, "{msg}");
            }
        }
        write!(f, "translaas API error: status {}", self.status_code)
    }
}

impl std::error::Error for ApiError {}

/// Invalid SDK configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct ConfigurationError {
    /// Description of the configuration problem.
    pub message: String,
}

/// Offline cache I/O or deserialization failure.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct OfflineCacheError {
    /// Human-readable error message.
    pub message: String,
    /// Cache directory path when known.
    pub cache_directory: Option<String>,
    /// Project id when known.
    pub project: Option<String>,
    /// Language code when known.
    pub language: Option<String>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl OfflineCacheError {
    /// Creates an offline cache error with optional underlying cause.
    pub fn new(
        message: impl Into<String>,
        cache_directory: Option<String>,
        project: Option<String>,
        language: Option<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            message: message.into(),
            cache_directory,
            project,
            language,
            source,
        }
    }
}

/// Expected data was not found in the offline cache.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct OfflineCacheMissError {
    /// Human-readable error message.
    pub message: String,
    /// Cache directory path when known.
    pub cache_directory: Option<String>,
    /// Project id when known.
    pub project: Option<String>,
    /// Language code when known.
    pub language: Option<String>,
    /// Translation group when known.
    pub group: Option<String>,
    /// Translation entry when known.
    pub entry: Option<String>,
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl OfflineCacheMissError {
    /// Builds a miss error with .NET-compatible messaging.
    pub fn new_offline_cache_miss_error(
        project: &str,
        language: &str,
        group: &str,
        entry: &str,
    ) -> Self {
        Self {
            message: build_offline_cache_miss_message(project, language, group, entry),
            cache_directory: None,
            project: Some(project.to_string()),
            language: Some(language.to_string()),
            group: if group.is_empty() {
                None
            } else {
                Some(group.to_string())
            },
            entry: if entry.is_empty() {
                None
            } else {
                Some(entry.to_string())
            },
            source: None,
        }
    }
}

fn build_offline_cache_miss_message(
    project: &str,
    language: &str,
    group: &str,
    entry: &str,
) -> String {
    if !entry.is_empty() && !group.is_empty() {
        return format!(
            "Translation entry '{entry}' in group '{group}' for project '{project}' and language '{language}' was not found in the offline cache."
        );
    }
    if !group.is_empty() {
        return format!(
            "Translation group '{group}' for project '{project}' and language '{language}' was not found in the offline cache."
        );
    }
    format!("Project '{project}' for language '{language}' was not found in the offline cache.")
}

/// Returned when language resolution yields no language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("no language could be resolved")]
pub struct NoLanguageError;

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> &'static str {
        match name {
            "translaas_error_full.json" => {
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/testdata/translaas_error_full.json"
                ))
            }
            _ => panic!("unknown fixture {name}"),
        }
    }

    #[test]
    fn translaas_error_format_message() {
        let err = TranslaasError {
            message: Some("Error message".to_string()),
            code: Some("ERROR_CODE".to_string()),
        };
        assert_eq!(err.format_message("fallback"), "[ERROR_CODE] Error message");

        let err = TranslaasError {
            message: Some("Error message".to_string()),
            code: None,
        };
        assert_eq!(err.format_message("fallback"), "Error message");

        let err = TranslaasError {
            message: None,
            code: Some("ERROR_CODE".to_string()),
        };
        assert_eq!(
            err.format_message("API request failed"),
            "[ERROR_CODE] API request failed"
        );
    }

    #[test]
    fn parse_translaas_error_golden() {
        let data = fixture("translaas_error_full.json");
        let parsed = parse_translaas_error(data.as_bytes()).unwrap().unwrap();
        assert_eq!(parsed.code.as_deref(), Some("ERROR_CODE"));
        assert_eq!(parsed.message.as_deref(), Some("Error message"));
    }

    #[test]
    fn offline_cache_miss_error_messages() {
        let err = OfflineCacheMissError::new_offline_cache_miss_error("p1", "en", "ui", "save");
        assert_eq!(
            err.to_string(),
            "Translation entry 'save' in group 'ui' for project 'p1' and language 'en' was not found in the offline cache."
        );

        let err = OfflineCacheMissError::new_offline_cache_miss_error("p1", "en", "ui", "");
        assert_eq!(
            err.to_string(),
            "Translation group 'ui' for project 'p1' and language 'en' was not found in the offline cache."
        );

        let err = OfflineCacheMissError::new_offline_cache_miss_error("p1", "en", "", "");
        assert_eq!(
            err.to_string(),
            "Project 'p1' for language 'en' was not found in the offline cache."
        );
    }

    #[test]
    fn api_error_display_prefers_message() {
        let err = ApiError {
            status_code: 404,
            code: None,
            message: Some("Not found".to_string()),
            response_content: None,
        };
        assert_eq!(err.to_string(), "Not found");

        let err = ApiError {
            status_code: 500,
            code: None,
            message: None,
            response_content: None,
        };
        assert_eq!(err.to_string(), "translaas API error: status 500");
    }
}
