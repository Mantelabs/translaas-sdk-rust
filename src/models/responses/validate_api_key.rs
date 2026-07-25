//! Response for `GET /api/v1/api-keys/validate`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Validate API key response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidateApiKeyResponse {
    /// Whether the API key is valid.
    pub is_valid: bool,
    /// Tenant id (flexible JSON shape).
    pub tenant_id: Option<serde_json::Value>,
    /// Project id (flexible JSON shape).
    pub project_id: Option<serde_json::Value>,
    /// All project ids accessible to the key.
    pub project_ids: Option<Vec<String>>,
    /// Default project id (flexible JSON shape).
    pub default_project_id: Option<serde_json::Value>,
    /// Integration name when present.
    pub integration_name: Option<String>,
    /// Authentication timestamp.
    pub authenticated_at: Option<DateTime<Utc>>,
}
