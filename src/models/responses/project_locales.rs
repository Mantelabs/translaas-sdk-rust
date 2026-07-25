//! Response for `GET /sdk/v1/translations/locales`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Project locales list returned by the locales endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectLocales {
    /// Project id.
    pub project: Option<String>,
    /// Supported locale codes.
    pub locales: Vec<String>,
    /// Last modification timestamp.
    pub last_modified_utc: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|_| panic!("read testdata {name}"))
    }

    #[test]
    fn project_locales_unmarshal() {
        let locales: ProjectLocales =
            serde_json::from_str(&fixture("project_locales.json")).unwrap();
        assert_eq!(locales.locales.len(), 4);
        assert_eq!(locales.locales[0], "en");
    }
}
