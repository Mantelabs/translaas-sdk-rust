//! Offline entry resolution: simplified plural rules and placeholder substitution.

use std::collections::HashMap;

use crate::models::{PluralCategory, TranslationGroup};

/// Resolves a rendered entry value from a cached group using offline plural and
/// placeholder rules (porting reference §8.3).
pub(crate) fn resolve_entry_from_group(
    group: &TranslationGroup,
    entry: &str,
    number: Option<f64>,
    params: &HashMap<String, String>,
) -> Option<String> {
    if group.has_plural_forms(entry) {
        let category = determine_plural_category(number);
        let mut form = group.get_plural_form(entry, category);
        if form.is_none() && category != PluralCategory::Other {
            form = group.get_plural_form(entry, PluralCategory::Other);
        }
        return form.map(|text| substitute_parameters(&text, number, params));
    }

    group
        .get_value(entry)
        .map(|value| substitute_parameters(value, number, params))
}

/// Offline plural selection: `n == 1` → `One`, otherwise `Other`.
pub(crate) fn determine_plural_category(number: Option<f64>) -> PluralCategory {
    match number {
        Some(1.0) => PluralCategory::One,
        _ => PluralCategory::Other,
    }
}

/// Substitutes `{parameter}` placeholders using case-insensitive parameter lookup.
pub(crate) fn substitute_parameters(
    template: &str,
    number: Option<f64>,
    params: &HashMap<String, String>,
) -> String {
    let merged = merge_number_into_parameters(number, params);
    if merged.is_empty() {
        return template.to_string();
    }

    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' {
            if let Some((name, end)) = parse_placeholder_name(&bytes[i + 1..]) {
                if let Some(value) = param_lookup(&merged, name) {
                    result.push_str(&value);
                } else {
                    result.push('{');
                    result.push_str(name);
                    result.push('}');
                }
                i += end + 2;
                continue;
            }
        }
        result.push(char::from(bytes[i]));
        i += 1;
    }
    result
}

fn parse_placeholder_name(rest: &[u8]) -> Option<(&str, usize)> {
    let mut end = 0;
    while end < rest.len() {
        let ch = rest[end];
        if ch == b'}' {
            if end == 0 {
                return None;
            }
            let name = std::str::from_utf8(&rest[..end]).ok()?;
            if name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Some((name, end));
            }
            return None;
        }
        if !(ch.is_ascii_alphanumeric() || ch == b'_') {
            return None;
        }
        end += 1;
    }
    None
}

fn merge_number_into_parameters(
    number: Option<f64>,
    params: &HashMap<String, String>,
) -> HashMap<String, String> {
    if number.is_none() && params.is_empty() {
        return HashMap::new();
    }

    let mut merged = params.clone();
    if let Some(n) = number {
        if !has_param_key(&merged, "N") {
            merged.insert("N".to_string(), format_plural_number(n));
        }
    }
    merged
}

fn format_plural_number(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
        return format!("{}", n as i64);
    }
    let mut s = format!("{n}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

fn has_param_key(params: &HashMap<String, String>, name: &str) -> bool {
    param_lookup(params, name).is_some()
}

fn param_lookup(params: &HashMap<String, String>, name: &str) -> Option<String> {
    params
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group_with_plural() -> TranslationGroup {
        let mut entries = HashMap::new();
        entries.insert(
            "items".to_string(),
            serde_json::json!({"One": "1 item", "Other": "{N} items"}),
        );
        TranslationGroup {
            entries,
            ..Default::default()
        }
    }

    #[test]
    fn substitute_parameters_simple() {
        let mut params = HashMap::new();
        params.insert("userName".to_string(), "John".to_string());
        assert_eq!(
            substitute_parameters("Hello {userName}", None, &params),
            "Hello John"
        );
    }

    #[test]
    fn substitute_parameters_number_injection() {
        assert_eq!(
            substitute_parameters("You have {N} items", Some(5.0), &HashMap::new()),
            "You have 5 items"
        );
    }

    #[test]
    fn substitute_parameters_number_and_params() {
        let mut params = HashMap::new();
        params.insert("userName".to_string(), "John".to_string());
        params.insert("pending".to_string(), "3".to_string());
        assert_eq!(
            substitute_parameters(
                "Hello {userName}, you have {N} items and {pending} pending",
                Some(5.0),
                &params
            ),
            "Hello John, you have 5 items and 3 pending"
        );
    }

    #[test]
    fn substitute_parameters_unknown_placeholder_preserved() {
        let mut params = HashMap::new();
        params.insert("userName".to_string(), "John".to_string());
        assert_eq!(
            substitute_parameters("Hello {unknown}", None, &params),
            "Hello {unknown}"
        );
    }

    #[test]
    fn substitute_parameters_case_insensitive_lookup() {
        let mut params = HashMap::new();
        params.insert("username".to_string(), "Jane".to_string());
        assert_eq!(
            substitute_parameters("Hello {UserName}", None, &params),
            "Hello Jane"
        );
    }

    #[test]
    fn determine_plural_category_rules() {
        assert_eq!(determine_plural_category(None), PluralCategory::Other);
        assert_eq!(determine_plural_category(Some(1.0)), PluralCategory::One);
        assert_eq!(determine_plural_category(Some(2.0)), PluralCategory::Other);
    }

    #[test]
    fn resolve_entry_from_group_plural() {
        let group = group_with_plural();
        assert_eq!(
            resolve_entry_from_group(&group, "items", Some(1.0), &HashMap::new()),
            Some("1 item".to_string())
        );
        assert_eq!(
            resolve_entry_from_group(&group, "items", Some(2.0), &HashMap::new()),
            Some("2 items".to_string())
        );
    }
}
