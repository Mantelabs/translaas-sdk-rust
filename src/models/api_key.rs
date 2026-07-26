//! API key validation helpers.

use crate::models::errors::ConfigurationError;
use crate::models::responses::ValidateApiKeyResponse;

/// Extracts a string ULID/id from flexible JSON element shapes.
pub fn read_json_ulid(raw: &serde_json::Value) -> Option<String> {
    match raw {
        serde_json::Value::Null => None,
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        other => {
            if let Ok(s) = serde_json::from_value::<String>(other.clone()) {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            } else {
                let trimmed = other.to_string().trim_matches('"').trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
        }
    }
}

/// Returns the effective default project id when not configured explicitly.
pub fn resolve_default_project_id(
    configured_project_id: &str,
    validate: &ValidateApiKeyResponse,
) -> Result<String, ConfigurationError> {
    let configured = configured_project_id.trim();
    if !configured.is_empty() {
        return Ok(configured.to_string());
    }

    if validate
        .project_ids
        .as_ref()
        .is_none_or(|ids| ids.is_empty())
    {
        return Err(ConfigurationError {
            message: "Tenant-level API key requires DefaultProjectId in SDK configuration."
                .to_string(),
        });
    }

    let mut from_validate = validate
        .default_project_id
        .as_ref()
        .and_then(read_json_ulid);
    if from_validate.is_none() {
        from_validate = validate.project_id.as_ref().and_then(read_json_ulid);
    }
    if from_validate.is_none() {
        if let Some(ids) = &validate.project_ids {
            if let Some(first) = ids.first() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    from_validate = Some(trimmed.to_string());
                }
            }
        }
    }

    from_validate.ok_or_else(|| ConfigurationError {
        message: "Could not resolve a default project from the validate API key response."
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::responses::ValidateApiKeyResponse;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|_| panic!("read testdata {name}"))
    }

    #[test]
    fn read_json_ulid_variants() {
        assert_eq!(
            read_json_ulid(&serde_json::json!("01ARZ3NDEKTSV4RRFFQ69G5FAV")),
            Some("01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string())
        );
        assert_eq!(read_json_ulid(&serde_json::Value::Null), None);
    }

    #[test]
    fn resolve_default_project_id_configured() {
        let got = resolve_default_project_id(
            "  my-project ",
            &ValidateApiKeyResponse {
                is_valid: false,
                tenant_id: None,
                project_id: None,
                project_ids: None,
                default_project_id: None,
                integration_name: None,
                authenticated_at: None,
            },
        )
        .unwrap();
        assert_eq!(got, "my-project");
    }

    #[test]
    fn resolve_default_project_id_from_validate() {
        let validate: ValidateApiKeyResponse =
            serde_json::from_str(&fixture("validate_api_key_tenant.json")).unwrap();
        let got = resolve_default_project_id("", &validate).unwrap();
        assert_eq!(got, "proj-a");
    }

    #[test]
    fn resolve_default_project_id_missing_project_ids() {
        let err = resolve_default_project_id(
            "",
            &ValidateApiKeyResponse {
                is_valid: true,
                tenant_id: None,
                project_id: None,
                project_ids: None,
                default_project_id: None,
                integration_name: None,
                authenticated_at: None,
            },
        )
        .unwrap_err();
        assert_eq!(
            err.message,
            "Tenant-level API key requires DefaultProjectId in SDK configuration."
        );
    }
}
