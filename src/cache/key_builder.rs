//! Colon-separated cache keys matching .NET / Go `CacheKeyBuilder`.

#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;

const KEY_SEPARATOR: &str = ":";

/// Produces cache keys byte-identical to .NET / Go `CacheKeyBuilder`.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyBuilder;

/// Builds a cache key for a single translation entry.
pub fn entry_key(
    group: &str,
    entry: &str,
    lang: &str,
    number: Option<f64>,
    parameters: &HashMap<String, String>,
    project: &str,
    channel: &str,
    version: &str,
) -> String {
    KeyBuilder.entry_key(
        group, entry, lang, number, parameters, project, channel, version,
    )
}

/// Builds a cache key for a translation group payload.
pub fn group_key(
    project: &str,
    group: &str,
    lang: &str,
    format: &str,
    channel: &str,
    version: &str,
    include_context: Option<bool>,
) -> String {
    KeyBuilder.group_key(
        project,
        group,
        lang,
        format,
        channel,
        version,
        include_context,
    )
}

/// Builds a cache key for a full project payload.
pub fn project_key(
    project: &str,
    lang: &str,
    format: &str,
    channel: &str,
    version: &str,
    include_context: Option<bool>,
) -> String {
    KeyBuilder.project_key(project, lang, format, channel, version, include_context)
}

/// Builds a cache key for project locales.
pub fn locales_key(project: &str, channel: &str, version: &str) -> String {
    KeyBuilder.locales_key(project, channel, version)
}

/// Builds a cache key for offline ZIP download metadata.
pub fn offline_key(
    project: &str,
    channel: &str,
    version: &str,
    include_context: Option<bool>,
) -> String {
    KeyBuilder.offline_key(project, channel, version, include_context)
}

impl KeyBuilder {
    /// Builds a cache key for a single translation entry.
    pub fn entry_key(
        &self,
        group: &str,
        entry: &str,
        lang: &str,
        number: Option<f64>,
        parameters: &HashMap<String, String>,
        project: &str,
        channel: &str,
        version: &str,
    ) -> String {
        let mut parts = vec![
            "entry".to_string(),
            group.to_string(),
            entry.to_string(),
            lang.to_string(),
        ];
        if let Some(n) = number {
            parts.push(format_cache_number(n));
        }
        parts.extend(sorted_param_pairs(parameters));
        append_snapshot_suffix(&parts, project, channel, version, None)
    }

    /// Builds a cache key for a translation group payload.
    pub fn group_key(
        &self,
        project: &str,
        group: &str,
        lang: &str,
        format: &str,
        channel: &str,
        version: &str,
        include_context: Option<bool>,
    ) -> String {
        let mut parts = vec![
            "group".to_string(),
            project.to_string(),
            group.to_string(),
            lang.to_string(),
        ];
        if !format.trim().is_empty() {
            parts.push(format.to_string());
        }
        append_snapshot_suffix(&parts, "", channel, version, include_context)
    }

    /// Builds a cache key for a full project payload.
    pub fn project_key(
        &self,
        project: &str,
        lang: &str,
        format: &str,
        channel: &str,
        version: &str,
        include_context: Option<bool>,
    ) -> String {
        let mut parts = vec!["project".to_string(), project.to_string(), lang.to_string()];
        if !format.trim().is_empty() {
            parts.push(format.to_string());
        }
        append_snapshot_suffix(&parts, "", channel, version, include_context)
    }

    /// Builds a cache key for project locales.
    pub fn locales_key(&self, project: &str, channel: &str, version: &str) -> String {
        let parts = vec!["locales".to_string(), project.to_string()];
        append_snapshot_suffix(&parts, "", channel, version, None)
    }

    /// Builds a cache key for offline ZIP download metadata.
    pub fn offline_key(
        &self,
        project: &str,
        channel: &str,
        version: &str,
        include_context: Option<bool>,
    ) -> String {
        let parts = vec!["offline".to_string(), project.to_string()];
        append_snapshot_suffix(&parts, "", channel, version, include_context)
    }
}

fn append_snapshot_suffix(
    parts: &[String],
    project: &str,
    channel: &str,
    version: &str,
    include_context: Option<bool>,
) -> String {
    let key = parts.join(KEY_SEPARATOR);
    let mut suffix_parts = Vec::with_capacity(4);
    if !project.trim().is_empty() {
        suffix_parts.push(format!("proj={project}"));
    }
    if !channel.trim().is_empty() {
        suffix_parts.push(format!("ch={channel}"));
    }
    if !version.trim().is_empty() {
        suffix_parts.push(format!("v={version}"));
    }
    if let Some(include_context) = include_context {
        suffix_parts.push(if include_context {
            "ic=1".to_string()
        } else {
            "ic=0".to_string()
        });
    }
    if suffix_parts.is_empty() {
        key
    } else {
        format!("{key}{KEY_SEPARATOR}{}", suffix_parts.join(KEY_SEPARATOR))
    }
}

fn sorted_param_pairs(parameters: &HashMap<String, String>) -> Vec<String> {
    if parameters.is_empty() {
        return Vec::new();
    }

    let mut keys: Vec<&String> = parameters
        .iter()
        .filter(|(key, value)| !key.trim().is_empty() && !value.trim().is_empty())
        .map(|(key, _)| key)
        .collect();

    keys.sort_by_key(|key| key.to_lowercase());

    keys.into_iter()
        .map(|key| format!("{}={}", key.to_lowercase(), parameters[key]))
        .collect()
}

/// Formats a plural/cache number matching Go `strconv.FormatFloat(n, 'g', 15, 64)`.
pub(crate) fn format_cache_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n.is_sign_negative() {
            "-Inf".to_string()
        } else {
            "+Inf".to_string()
        };
    }
    if n == 0.0 {
        return "0".to_string();
    }

    let sign = if n.is_sign_negative() { "-" } else { "" };
    let abs = n.abs();

    let e_fmt = format_go_exp(abs, 15);
    let f_fmt = format_go_fixed(abs, 15);

    let body = if e_fmt.len() <= f_fmt.len() {
        e_fmt
    } else {
        f_fmt
    };

    format!("{sign}{body}")
}

fn format_go_fixed(mut n: f64, prec: i32) -> String {
    // Round to prec significant digits in fixed notation.
    if n == 0.0 {
        return "0".to_string();
    }

    let exp = n.log10().floor() as i32;
    let scale = 10_f64.powi(prec - 1 - exp);
    n = (n * scale).round() / scale;

    let mut s = format!("{n}");
    trim_trailing_zeros(&mut s);
    s
}

fn format_go_exp(n: f64, prec: i32) -> String {
    if n == 0.0 {
        return "0".to_string();
    }

    let exp = n.log10().floor() as i32;
    let mantissa = n / 10_f64.powi(exp);
    let scale = 10_f64.powi(prec - 1);
    let rounded = (mantissa * scale).round() / scale;

    let mut mantissa_str = format!("{rounded}");
    trim_trailing_zeros(&mut mantissa_str);

    let exp_abs = exp.unsigned_abs();
    if exp >= 0 {
        format!("{mantissa_str}e+{exp_abs:02}")
    } else {
        format!("{mantissa_str}e-{exp_abs:02}")
    }
}

fn trim_trailing_zeros(s: &mut String) {
    if !s.contains('.') {
        return;
    }
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{entry_key, format_cache_number, group_key, offline_key, KeyBuilder};

    #[test]
    fn package_level_entry_key_helper() {
        assert_eq!(
            entry_key("ui", "save", "en", None, &HashMap::new(), "", "", ""),
            "entry:ui:save:en"
        );
    }

    #[test]
    fn format_cache_number_matches_go() {
        let cases = [
            (1.0, "1"),
            (1.0_f64, "1"),
            (1.5, "1.5"),
            (0.0000001, "1e-07"),
            (1e15, "1e+15"),
        ];
        for (n, want) in cases {
            assert_eq!(format_cache_number(n), want, "n={n}");
        }
    }

    #[test]
    fn include_context_suffixes() {
        let ic_true = group_key("p", "g", "en", "", "", "", Some(true));
        assert!(ic_true.ends_with(":ic=1"));

        let ic_false = group_key("p", "g", "en", "", "", "", Some(false));
        assert!(ic_false.ends_with(":ic=0"));

        let offline = offline_key("proj", "", "", Some(false));
        assert_eq!(offline, "offline:proj:ic=0");
    }

    #[test]
    fn param_sorting_is_case_insensitive_on_key() {
        let mut params = HashMap::new();
        params.insert("Zebra".to_string(), "1".to_string());
        params.insert("foo".to_string(), "bar".to_string());
        let key = KeyBuilder.entry_key("g", "e", "en", None, &params, "", "", "");
        assert_eq!(key, "entry:g:e:en:foo=bar:zebra=1");
    }
}
