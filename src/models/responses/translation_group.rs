//! Translation group response with dual JSON shapes.

use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::models::plural::{parse_plural_category, PluralCategory};
use crate::models::serde_utils::{raw_string, read_entry_context};

/// Translation group with one or more entries.
///
/// Supports two JSON shapes:
/// - **Full API**: metadata fields plus an `Entries` object.
/// - **Flat offline/cache**: root object is the entries map.
///
/// Serialization always emits the canonical full write shape.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TranslationGroup {
    /// Project id (`Project` on the wire).
    pub project: Option<String>,
    /// Language code (`Lang` on the wire).
    pub lang: Option<String>,
    /// Version (`Version` on the wire; string or number).
    pub version: Option<Value>,
    /// Generation timestamp (`GeneratedAt` on the wire).
    pub generated_at: Option<DateTime<Utc>>,
    /// Entry key to value map (`Entries` on the wire).
    pub entries: HashMap<String, Value>,
    /// Optional entry context (`entryContext` / `EntryContext` on the wire).
    pub entry_context: Option<HashMap<String, Value>>,
}

impl TranslationGroup {
    /// Returns a simple string entry value when present and not plural.
    pub fn get_value(&self, key: &str) -> Option<&str> {
        match self.entries.get(key)? {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Reports whether an entry is a plural object map.
    pub fn has_plural_forms(&self, key: &str) -> bool {
        matches!(self.entries.get(key), Some(Value::Object(_)))
    }

    /// Returns plural category values for an entry.
    pub fn get_plural_forms(&self, key: &str) -> Option<HashMap<PluralCategory, String>> {
        let raw = self.entries.get(key)?;
        let obj = raw.as_object()?;
        let mut out = HashMap::new();
        for (name, value) in obj {
            let category = parse_plural_category(name)?;
            let text = value.as_str()?.to_string();
            out.insert(category, text);
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// Returns one plural category value for an entry.
    pub fn get_plural_form(&self, key: &str, category: PluralCategory) -> Option<String> {
        let raw = self.entries.get(key)?;
        let obj = raw.as_object()?;
        for (name, value) in obj {
            if parse_plural_category(name)? == category {
                return value.as_str().map(str::to_string);
            }
        }
        None
    }
}

impl<'de> Deserialize<'de> for TranslationGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TranslationGroupVisitor)
    }
}

struct TranslationGroupVisitor;

impl<'de> Visitor<'de> for TranslationGroupVisitor {
    type Value = TranslationGroup;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a translation group object")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut root = serde_json::Map::new();
        while let Some((key, value)) = map.next_entry::<String, Value>()? {
            root.insert(key, value);
        }

        let entry_context = read_entry_context(&root);
        let mut group = TranslationGroup {
            entry_context,
            ..Default::default()
        };

        if let Some(entries_raw) = root.get("Entries") {
            group.project = root.get("Project").and_then(raw_string);
            group.lang = root.get("Lang").and_then(raw_string);
            group.version = root.get("Version").cloned();
            group.generated_at = root
                .get("GeneratedAt")
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            group.entries =
                serde_json::from_value(entries_raw.clone()).map_err(de::Error::custom)?;
            return Ok(group);
        }

        group.entries = root
            .into_iter()
            .filter(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "entryContext"
                        | "EntryContext"
                        | "Project"
                        | "Lang"
                        | "Version"
                        | "GeneratedAt"
                )
            })
            .collect();
        Ok(group)
    }
}

impl Serialize for TranslationGroup {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(None)?;

        if let Some(ref project) = self.project {
            map.serialize_entry("Project", project)?;
        }
        if let Some(ref lang) = self.lang {
            map.serialize_entry("Lang", lang)?;
        }
        if let Some(ref version) = self.version {
            map.serialize_entry("Version", version)?;
        }
        if let Some(ref generated_at) = self.generated_at {
            map.serialize_entry("GeneratedAt", generated_at)?;
        }
        if let Some(ref ctx) = self.entry_context {
            if !ctx.is_empty() {
                map.serialize_entry("entryContext", ctx)?;
            }
        }
        map.serialize_entry("Entries", &self.entries)?;
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
    fn translation_group_flat_simple() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_flat_simple.json")).unwrap();
        assert_eq!(group.get_value("button.save"), Some("Save"));
        assert!(!group.has_plural_forms("button.save"));
    }

    #[test]
    fn translation_group_empty() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_empty.json")).unwrap();
        assert!(group.entries.is_empty());
    }

    #[test]
    fn translation_group_plural_en() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_plural_en.json")).unwrap();
        assert!(group.has_plural_forms("simple-count"));
        assert_eq!(group.get_value("simple-count"), None);
        let forms = group.get_plural_forms("simple-count").unwrap();
        assert_eq!(
            forms.get(&PluralCategory::One).map(String::as_str),
            Some("There is 1 record")
        );
        assert_eq!(
            forms.get(&PluralCategory::Other).map(String::as_str),
            Some("There are {0} records")
        );
        assert_eq!(
            group.get_plural_form("simple-count", PluralCategory::One),
            Some("There is 1 record".to_string())
        );
    }

    #[test]
    fn translation_group_plural_ar() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_plural_ar.json")).unwrap();
        for cat in [
            PluralCategory::Zero,
            PluralCategory::One,
            PluralCategory::Two,
            PluralCategory::Few,
            PluralCategory::Many,
            PluralCategory::Other,
        ] {
            assert!(
                group.get_plural_form("item", cat).is_some(),
                "missing category {cat:?}"
            );
        }
    }

    #[test]
    fn translation_group_full_api() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_full_api.json")).unwrap();
        assert_eq!(group.project.as_deref(), Some("my-project"));
        assert_eq!(group.lang.as_deref(), Some("en"));
        assert!(group.generated_at.is_some());
        assert_eq!(group.entry_context.as_ref().map(HashMap::len), Some(1));
        assert_eq!(group.get_value("welcome"), Some("Welcome"));

        let round_trip = serde_json::to_string(&group).unwrap();
        let again: TranslationGroup = serde_json::from_str(&round_trip).unwrap();
        assert_eq!(again.project, group.project);
        assert_eq!(again.entries.len(), group.entries.len());
    }

    #[test]
    fn translation_group_mixed() {
        let group: TranslationGroup =
            serde_json::from_str(&fixture("translation_group_mixed.json")).unwrap();
        assert!(group.has_plural_forms("simple-count"));
        assert_eq!(group.get_value("button.save"), Some("Save"));
    }
}
