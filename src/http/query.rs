//! Query-string construction from request DTOs and extra parameters.

use std::collections::HashMap;

use crate::models::ConfigurationError;
use serde::Serialize;
use serde_json::Value;
use url::Url;

/// Appends query parameters by serializing `params` and using JSON field names.
///
/// Null, empty strings, arrays, and objects are omitted. Booleans and numbers
/// are always included when present, including `false` and `0`.
pub(crate) fn append_query_values(
    url: &mut Url,
    params: &impl Serialize,
) -> Result<(), ConfigurationError> {
    let value = serde_json::to_value(params).map_err(|err| ConfigurationError {
        message: format!("http: serialize query params: {err}"),
    })?;

    let object = value.as_object().ok_or_else(|| ConfigurationError {
        message: "http: query params must serialize to a JSON object".to_string(),
    })?;

    let mut current = parse_query_params(url.query().unwrap_or_default());
    for (key, field_value) in object {
        let Some(formatted) = value_to_query_string(field_value) else {
            continue;
        };
        upsert_query_param(&mut current, key, &formatted);
    }
    set_query_params(url, &current);
    Ok(())
}

/// Merges `extra` into the URL query. On case-insensitive key collision the
/// existing parameter is replaced using `extra`'s key casing. Empty values are skipped.
pub(crate) fn merge_query_params(url: &mut Url, extra: &HashMap<String, String>) {
    if extra.is_empty() {
        return;
    }

    let mut current = parse_query_params(url.query().unwrap_or_default());
    for (key, value) in extra {
        if value.trim().is_empty() {
            continue;
        }
        upsert_query_param(&mut current, key, value);
    }
    set_query_params(url, &current);
}

/// Injects capital-`N` into `extra` when `n` is present and no case-insensitive
/// `N` key already exists in the map.
pub(crate) fn inject_plural_n(extra: &mut HashMap<String, String>, n: Option<f64>) {
    let Some(n) = n else {
        return;
    };

    if extra.keys().any(|key| key.eq_ignore_ascii_case("N")) {
        return;
    }

    extra.insert("N".to_string(), format_float(n));
}

/// Returns decoded query parameters using the first value for each key.
pub(crate) fn query_values(url: &Url) -> HashMap<String, String> {
    parse_query_params(url.query().unwrap_or_default())
        .into_iter()
        .collect()
}

fn parse_query_params(raw_query: &str) -> Vec<(String, String)> {
    if raw_query.is_empty() {
        return Vec::new();
    }

    url::form_urlencoded::parse(raw_query.as_bytes())
        .into_owned()
        .collect()
}

fn upsert_query_param(params: &mut Vec<(String, String)>, key: &str, value: &str) {
    params.retain(|(existing, _)| !existing.eq_ignore_ascii_case(key));
    params.push((key.to_string(), value.to_string()));
}

fn set_query_params(url: &mut Url, params: &[(String, String)]) {
    if params.is_empty() {
        url.set_query(None);
        return;
    }

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (key, value) in params {
        serializer.append_pair(key, value);
    }
    url.set_query(Some(&serializer.finish()));
}

fn value_to_query_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) if text.is_empty() => None,
        Value::String(text) => Some(text.clone()),
        Value::Bool(boolean) => Some(boolean.to_string()),
        Value::Number(number) => number
            .as_f64()
            .map(format_float)
            .or_else(|| Some(number.to_string())),
        Value::Array(_) | Value::Object(_) => None,
    }
}

fn format_float(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }

    let mut formatted = format!("{value:.14}");
    if formatted.contains('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
    }
    formatted
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::models::{GetGroupTranslationsRequest, GetTranslationRequest};

    fn parse_test_url(raw: &str) -> Url {
        Url::parse(raw).expect("valid test url")
    }

    fn assert_query_equal(got: &Url, want: &Url) {
        let got_values = query_values(got);
        let want_values = query_values(want);
        assert_eq!(got_values.len(), want_values.len(), "query length mismatch");
        for (key, value) in want_values {
            assert_eq!(
                got_values.get(&key),
                Some(&value),
                "key {key} mismatch in {}",
                got.query().unwrap_or_default()
            );
        }
        assert_eq!(got.path(), want.path());
    }

    #[test]
    fn append_query_values_get_translation_request() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        let req = GetTranslationRequest {
            group: Some("ui".to_string()),
            entry: Some("button.save".to_string()),
            lang: Some("en".to_string()),
            n: None,
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();

        let want = parse_test_url(
            "https://api.test.com/sdk/v1/translations/text?group=ui&entry=button.save&lang=en",
        );
        assert_query_equal(&url, &want);
    }

    #[test]
    fn append_query_values_includes_number_and_decimal() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");

        let req = GetTranslationRequest {
            group: None,
            entry: None,
            lang: None,
            n: Some(5.0),
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();
        assert_eq!(query_values(&url).get("n"), Some(&"5".to_string()));

        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        let req = GetTranslationRequest {
            group: None,
            entry: None,
            lang: None,
            n: Some(1.31),
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();
        assert_eq!(query_values(&url).get("n"), Some(&"1.31".to_string()));
    }

    #[test]
    fn append_query_values_includes_zero_and_false() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        let req = GetTranslationRequest {
            group: None,
            entry: None,
            lang: None,
            n: Some(0.0),
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();
        assert_eq!(query_values(&url).get("n"), Some(&"0".to_string()));

        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/group");
        let req = GetGroupTranslationsRequest {
            project: None,
            group: None,
            lang: None,
            format: None,
            channel: None,
            version: None,
            include_context: Some(false),
        };
        append_query_values(&mut url, &req).unwrap();
        assert_eq!(
            query_values(&url).get("includeContext"),
            Some(&"false".to_string())
        );
    }

    #[test]
    fn append_query_values_omits_empty_request() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        let req = GetTranslationRequest {
            group: None,
            entry: None,
            lang: None,
            n: None,
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();
        assert!(url.query().is_none());
    }

    #[test]
    fn append_query_values_rejects_non_object() {
        let mut url = parse_test_url("https://api.test.com/");
        assert!(append_query_values(&mut url, &"bad").is_err());
    }

    #[test]
    fn merge_query_params_replaces_case_insensitively() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text?group=ui&n=5");
        merge_query_params(
            &mut url,
            &HashMap::from([("N".to_string(), "5".to_string())]),
        );

        let values = query_values(&url);
        assert_eq!(values.get("N"), Some(&"5".to_string()));
        assert!(!values.contains_key("n"));
        assert_eq!(values.get("group"), Some(&"ui".to_string()));
    }

    #[test]
    fn merge_query_params_url_encodes_special_characters() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        merge_query_params(
            &mut url,
            &HashMap::from([
                ("userName".to_string(), "John Doe".to_string()),
                ("message".to_string(), "Hello & Welcome".to_string()),
            ]),
        );

        let values = query_values(&url);
        assert_eq!(values.get("userName"), Some(&"John Doe".to_string()));
        assert_eq!(values.get("message"), Some(&"Hello & Welcome".to_string()));
    }

    #[test]
    fn merge_query_params_skips_empty_values() {
        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text?group=ui");
        merge_query_params(
            &mut url,
            &HashMap::from([
                ("userName".to_string(), String::new()),
                ("N".to_string(), "5".to_string()),
            ]),
        );

        let values = query_values(&url);
        assert!(!values.contains_key("userName"));
        assert_eq!(values.get("N"), Some(&"5".to_string()));
    }

    #[test]
    fn inject_plural_n_and_get_entry_flow() {
        let mut extra = HashMap::new();
        inject_plural_n(&mut extra, Some(5.0));
        assert_eq!(extra.get("N"), Some(&"5".to_string()));

        let mut existing = HashMap::from([("n".to_string(), "10".to_string())]);
        inject_plural_n(&mut existing, Some(5.0));
        assert!(!existing.contains_key("N"));

        let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
        let req = GetTranslationRequest {
            group: Some("ui".to_string()),
            entry: Some("button.save".to_string()),
            lang: Some("en".to_string()),
            n: Some(5.0),
            project: None,
            channel: None,
            version: None,
        };
        append_query_values(&mut url, &req).unwrap();

        let mut extra = HashMap::from([("userName".to_string(), "John".to_string())]);
        inject_plural_n(&mut extra, req.n);
        merge_query_params(&mut url, &extra);

        let values = query_values(&url);
        assert_eq!(values.get("N"), Some(&"5".to_string()));
        assert!(!values.contains_key("n"));
        assert_eq!(values.get("userName"), Some(&"John".to_string()));
    }

    #[test]
    fn query_order_independence() {
        let build = || {
            let mut url = parse_test_url("https://api.test.com/sdk/v1/translations/text");
            let req = GetTranslationRequest {
                group: Some("ui".to_string()),
                entry: Some("button.save".to_string()),
                lang: Some("en".to_string()),
                n: None,
                project: None,
                channel: None,
                version: None,
            };
            append_query_values(&mut url, &req).unwrap();
            merge_query_params(
                &mut url,
                &HashMap::from([
                    ("userName".to_string(), "John".to_string()),
                    ("N".to_string(), "5".to_string()),
                ]),
            );
            url
        };

        assert_query_equal(&build(), &build());
    }
}
