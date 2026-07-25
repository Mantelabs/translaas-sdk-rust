//! Translation project response with flexible group keys.

use std::collections::HashMap;
use std::fmt;

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use super::TranslationGroup;

/// Project payload with dynamic group keys at the JSON root.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TranslationProject {
    /// Optional group-level entry context.
    pub group_entry_context: Option<HashMap<String, Value>>,
    /// Raw group JSON blobs keyed by group name.
    pub groups: HashMap<String, Value>,
}

impl TranslationProject {
    /// Returns a group by name, supporting API and flat offline shapes.
    pub fn get_group(
        &self,
        group_name: &str,
    ) -> Result<Option<TranslationGroup>, serde_json::Error> {
        let Some(raw) = self.groups.get(group_name) else {
            return Ok(None);
        };

        if raw.is_null() {
            return Ok(Some(TranslationGroup::default()));
        }

        if !raw.is_object() {
            let group: TranslationGroup = serde_json::from_value(raw.clone())?;
            return Ok(Some(group));
        }

        let obj = raw.as_object().expect("checked is_object");
        if obj.contains_key("Entries") {
            let group: TranslationGroup = serde_json::from_value(raw.clone())?;
            return Ok(Some(group));
        }

        let entries: HashMap<String, Value> = serde_json::from_value(raw.clone())?;
        Ok(Some(TranslationGroup {
            entries,
            ..Default::default()
        }))
    }
}

impl<'de> Deserialize<'de> for TranslationProject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TranslationProjectVisitor)
    }
}

struct TranslationProjectVisitor;

impl<'de> Visitor<'de> for TranslationProjectVisitor {
    type Value = TranslationProject;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a translation project object")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut project = TranslationProject::default();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            if key == "groupEntryContext" {
                project.group_entry_context =
                    Some(serde_json::from_value(value).map_err(de::Error::custom)?);
            } else {
                project.groups.insert(key, value);
            }
        }
        Ok(project)
    }
}

impl Serialize for TranslationProject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in &self.groups {
            map.serialize_entry(key, value)?;
        }
        if let Some(ref ctx) = self.group_entry_context {
            if !ctx.is_empty() {
                map.serialize_entry("groupEntryContext", ctx)?;
            }
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> String {
        std::fs::read_to_string(format!("{}/testdata/{name}", env!("CARGO_MANIFEST_DIR")))
            .unwrap_or_else(|_| panic!("read testdata {name}"))
    }

    #[test]
    fn translation_project_flat_groups() {
        let project: TranslationProject =
            serde_json::from_str(&fixture("translation_project_flat.json")).unwrap();
        let ui = project.get_group("ui").unwrap().unwrap();
        assert_eq!(ui.get_value("button.save"), Some("Save"));
        let common = project.get_group("common").unwrap().unwrap();
        assert_eq!(common.get_value("welcome"), Some("Welcome"));
    }

    #[test]
    fn translation_project_api_group_shape() {
        let raw = r#"{
            "ui": {
                "Project": "p",
                "Lang": "en",
                "Entries": { "save": "Save" }
            }
        }"#;
        let project: TranslationProject = serde_json::from_str(raw).unwrap();
        let group = project.get_group("ui").unwrap().unwrap();
        assert_eq!(group.get_value("save"), Some("Save"));
    }

    #[test]
    fn translation_project_group_entry_context() {
        let raw = r#"{
            "groupEntryContext": { "ui": { "note": "ctx" } },
            "ui": { "hello": "Hello" }
        }"#;
        let project: TranslationProject = serde_json::from_str(raw).unwrap();
        assert_eq!(
            project.group_entry_context.as_ref().map(HashMap::len),
            Some(1)
        );
        assert_eq!(project.groups.len(), 1);

        let encoded = serde_json::to_string(&project).unwrap();
        let again: TranslationProject = serde_json::from_str(&encoded).unwrap();
        assert_eq!(
            again.group_entry_context.as_ref().map(HashMap::len),
            Some(1)
        );
        assert_eq!(again.groups.len(), 1);
    }
}
