//! Accept-Language parsing and ISO 639-1 normalization (Go parity).

/// Extracts the primary ISO 639-1 code from an `Accept-Language` header.
pub fn parse_accept_language(accept_language: &str) -> String {
    if accept_language.is_empty() {
        return String::new();
    }

    let first = accept_language
        .split(',')
        .next()
        .map(str::trim)
        .unwrap_or_default();

    if first.is_empty() {
        return String::new();
    }

    let tag = first.split(';').next().map(str::trim).unwrap_or_default();
    normalize_language_code(tag)
}

/// Converts a language tag to ISO 639-1 when possible.
pub fn normalize_language_code(lang: &str) -> String {
    let lang = lang.trim().to_ascii_lowercase();
    if lang.is_empty() {
        return String::new();
    }

    if lang.len() == 2 && is_alpha2(&lang) {
        return lang;
    }

    if lang.len() >= 2 && is_alpha2(&lang[..2]) {
        let rest = &lang[2..];
        if rest.is_empty() {
            return lang[..2].to_string();
        }
        if rest.starts_with('-') || rest.starts_with('_') {
            let suffix = &rest[1..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_alphabetic()) {
                return lang[..2].to_string();
            }
        }
    }

    String::new()
}

fn is_alpha2(code: &str) -> bool {
    code.len() == 2 && code.chars().all(|c| c.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{normalize_language_code, parse_accept_language};

    #[test]
    fn parse_accept_language_cases() {
        let cases = [
            ("primary with region", "en-US,en;q=0.9", "en"),
            ("quality suffix only", "fr;q=0.8", "fr"),
            ("simple code", "de", "de"),
            ("empty", "", ""),
            ("invalid", "invalid", ""),
        ];

        for (name, header, want) in cases {
            assert_eq!(
                parse_accept_language(header),
                want,
                "case {name}: parse_accept_language({header:?})"
            );
        }
    }

    #[test]
    fn normalize_language_code_cases() {
        let cases = [
            ("en", "en"),
            ("EN-US", "en"),
            ("fr_FR", "fr"),
            ("  pt  ", "pt"),
            ("invalid", ""),
        ];

        for (input, want) in cases {
            assert_eq!(
                normalize_language_code(input),
                want,
                "normalize_language_code({input:?})"
            );
        }
    }
}
