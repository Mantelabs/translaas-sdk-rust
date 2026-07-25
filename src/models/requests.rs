//! Request query and body models for SDK HTTP calls.

use serde::{Deserialize, Serialize};

/// Query model for `GET /sdk/v1/translations/text`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTranslationRequest {
    /// Translation group key.
    pub group: Option<String>,
    /// Translation entry key.
    pub entry: Option<String>,
    /// Language ISO code.
    pub lang: Option<String>,
    /// Plural count (`n` query parameter).
    pub n: Option<f64>,
    /// Project id.
    pub project: Option<String>,
    /// Channel.
    pub channel: Option<String>,
    /// Version (`v` query parameter).
    #[serde(rename = "v")]
    pub version: Option<String>,
}

/// Query model for `GET /sdk/v1/translations/group`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGroupTranslationsRequest {
    /// Project id.
    pub project: Option<String>,
    /// Translation group key.
    pub group: Option<String>,
    /// Language ISO code.
    pub lang: Option<String>,
    /// Response format.
    pub format: Option<String>,
    /// Channel.
    pub channel: Option<String>,
    /// Version (`v` query parameter).
    #[serde(rename = "v")]
    pub version: Option<String>,
    /// Whether to include entry context.
    pub include_context: Option<bool>,
}

/// Query model for `GET /sdk/v1/translations/project`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectTranslationsRequest {
    /// Project id.
    pub project: Option<String>,
    /// Language ISO code.
    pub lang: Option<String>,
    /// Response format.
    pub format: Option<String>,
    /// Channel.
    pub channel: Option<String>,
    /// Version (`v` query parameter).
    #[serde(rename = "v")]
    pub version: Option<String>,
    /// Whether to include entry context.
    pub include_context: Option<bool>,
}

/// Query model for `GET /sdk/v1/translations/locales`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProjectLocalesRequest {
    /// Project id.
    pub project: Option<String>,
    /// Channel.
    pub channel: Option<String>,
    /// Version (`v` query parameter).
    #[serde(rename = "v")]
    pub version: Option<String>,
}

/// Query model for `GET /sdk/v1/translations/offline-cache`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOfflineCacheRequest {
    /// Project id.
    pub project: Option<String>,
    /// Channel.
    pub channel: Option<String>,
    /// Version (`v` query parameter).
    #[serde(rename = "v")]
    pub version: Option<String>,
    /// Whether to include entry context.
    pub include_context: Option<bool>,
}

/// One missing key for `POST /sdk/v1/translations/report-missing`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMissingKeyItem {
    /// Group key.
    pub group_key: String,
    /// Entry key.
    pub entry_key: String,
    /// Language ISO code.
    pub language_iso_code: String,
}

/// POST body for reporting missing translation keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMissingKeysRequest {
    /// Missing keys to report.
    pub keys: Vec<ReportMissingKeyItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_translation_request_serializes_camel_case() {
        let req = GetTranslationRequest {
            group: Some("ui".to_string()),
            entry: Some("button.save".to_string()),
            lang: Some("en".to_string()),
            n: Some(5.0),
            project: Some("my-project".to_string()),
            channel: Some("stable".to_string()),
            version: Some("42".to_string()),
        };
        let value: serde_json::Value = serde_json::to_value(&req).unwrap();
        let obj = value.as_object().unwrap();
        for key in ["group", "entry", "lang", "n", "project", "channel", "v"] {
            assert!(obj.contains_key(key), "missing key {key}");
        }
        assert!(!obj.contains_key("group_key"));
    }

    #[test]
    fn report_missing_keys_request_round_trip() {
        let req = ReportMissingKeysRequest {
            keys: vec![ReportMissingKeyItem {
                group_key: "ui".to_string(),
                entry_key: "missing.key".to_string(),
                language_iso_code: "en".to_string(),
            }],
        };
        let json = serde_json::to_string(&req).unwrap();
        let decoded: ReportMissingKeysRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.keys.len(), 1);
        assert_eq!(decoded.keys[0].group_key, "ui");
    }
}
