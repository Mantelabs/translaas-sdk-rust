//! Shared Serde helpers for model wire formats.

use std::collections::HashMap;

use serde_json::Value;

/// Reads `entryContext` or `EntryContext` from a JSON object root.
pub(crate) fn read_entry_context(
    root: &serde_json::Map<String, Value>,
) -> Option<HashMap<String, Value>> {
    for name in ["entryContext", "EntryContext"] {
        if let Some(raw) = root.get(name) {
            if let Ok(ctx) = serde_json::from_value::<HashMap<String, Value>>(raw.clone()) {
                return Some(ctx);
            }
        }
    }
    None
}

/// Extracts a string from a flexible JSON value (string or raw fragment).
pub(crate) fn raw_string(raw: &Value) -> Option<String> {
    match raw {
        Value::String(s) => Some(s.clone()),
        Value::Null => None,
        other => {
            if let Ok(s) = serde_json::from_value::<String>(other.clone()) {
                Some(s)
            } else {
                Some(other.to_string().trim_matches('"').to_string())
            }
        }
    }
}
