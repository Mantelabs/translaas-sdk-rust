//! Path sanitization and offline cache error helpers.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::models::OfflineCacheError;

/// Replaces invalid filename characters with `_` for cache directory names.
pub fn sanitize_path_segment(name: &str) -> Result<String, String> {
    validate_segment_input(name)?;

    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if is_invalid_filename_rune(ch) {
            out.push('_');
        } else {
            out.push(ch);
        }
    }

    let trimmed = out.trim();
    if trimmed.is_empty() {
        return Err("path segment is empty after sanitization".to_string());
    }

    Ok(trimmed.to_string())
}

pub(crate) fn is_expired(expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    expires_at.is_some_and(|expiry| expiry < now)
}

pub(crate) fn offline_cache_err(
    dir: &Path,
    project: &str,
    lang: &str,
    message: impl Into<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
) -> OfflineCacheError {
    OfflineCacheError::new(
        message,
        Some(dir.display().to_string()),
        optional_string(project),
        optional_string(lang),
        source,
    )
}

pub(crate) fn check_cancelled(
    dir: &Path,
    project: &str,
    lang: &str,
    cancelled: bool,
) -> Result<(), OfflineCacheError> {
    if cancelled {
        return Err(offline_cache_err(
            dir,
            project,
            lang,
            "operation cancelled",
            None,
        ));
    }
    Ok(())
}

pub(crate) fn resolve_absolute_path(path: &Path) -> Result<PathBuf, OfflineCacheError> {
    if path.as_os_str().is_empty() {
        return Err(offline_cache_err(
            path,
            "",
            "",
            "cache directory must not be empty",
            None,
        ));
    }

    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let cwd = std::env::current_dir().map_err(|err| {
        offline_cache_err(
            path,
            "",
            "",
            format!("resolve cache directory: {err}"),
            Some(Box::new(err)),
        )
    })?;

    Ok(cwd.join(path))
}

fn validate_segment_input(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("path segment must not be empty".to_string());
    }
    if name.contains("..") {
        return Err("path segment must not contain '..'".to_string());
    }
    if Path::new(name).is_absolute() {
        return Err("path segment must not be absolute".to_string());
    }
    Ok(())
}

fn is_invalid_filename_rune(ch: char) -> bool {
    if ch.is_ascii_control() {
        return true;
    }
    matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
}

fn optional_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_path_segment;

    #[test]
    fn sanitize_path_segment_cases() {
        let cases = [
            ("simple", "my-project", Ok("my-project".to_string())),
            ("replace slash", "my/project", Ok("my_project".to_string())),
            ("replace colon", "my:project", Ok("my_project".to_string())),
            (
                "empty",
                "   ",
                Err("path segment must not be empty".to_string()),
            ),
            (
                "traversal",
                "..",
                Err("path segment must not contain '..'".to_string()),
            ),
            (
                "embedded traversal",
                "foo..bar",
                Err("path segment must not contain '..'".to_string()),
            ),
        ];

        for (name, input, want) in cases {
            let got = sanitize_path_segment(input).map_err(|err| err.to_string());
            assert_eq!(got, want, "case {name}");
        }
    }
}
