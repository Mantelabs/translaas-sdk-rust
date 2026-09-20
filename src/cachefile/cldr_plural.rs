//! CLDR cardinal plural selection for offline / file-cache entry resolution.
//!
//! Uses ICU4X [`icu_plurals::PluralRules`] so locales such as Arabic, Polish, and
//! French select `zero` / `two` / `few` / `one` instead of an English-like
//! `n == 1` heuristic. Pass a full BCP-47 tag (`pt` vs `pt-PT`). Invalid or empty
//! language tags fall back to the base language, then `en`. Live HTTP `get_entry`
//! is unchanged — the server still selects from `n`.
//!
//! Compiled CLDR data is not `Send`/`Sync`, so resolved [`PluralRules`] are cached
//! per thread.

use std::cell::RefCell;
use std::collections::HashMap;

use icu_locale::Locale;
use icu_plurals::{PluralOperands, PluralRules};

use crate::models::PluralCategory;

const ENGLISH_LOCALE: &str = "en";

thread_local! {
    static RULES_CACHE: RefCell<HashMap<String, PluralRules>> = RefCell::new(HashMap::new());
}

/// Resolves the CLDR cardinal category for `number` and `lang`.
///
/// When `number` is `None` or non-finite, returns [`PluralCategory::Other`].
/// Empty or invalid language tags fall back to English CLDR rules.
pub(crate) fn resolve_category(number: Option<f64>, lang: &str) -> PluralCategory {
    let Some(n) = number else {
        return PluralCategory::Other;
    };
    if !n.is_finite() {
        return PluralCategory::Other;
    }

    with_plural_rules(lang, |rules| {
        map_icu_category(category_for_number(rules, n))
    })
}

fn with_plural_rules<T>(lang: &str, f: impl FnOnce(&PluralRules) -> T) -> T {
    let locale = normalize_locale_tag(lang);
    let cache_key = locale.to_ascii_lowercase();

    RULES_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if !cache.contains_key(&cache_key) {
            cache.insert(cache_key.clone(), create_plural_rules(&locale));
        }
        f(cache
            .get(&cache_key)
            .expect("rules were just inserted for this locale"))
    })
}

fn category_for_number(rules: &PluralRules, n: f64) -> icu_plurals::PluralCategory {
    if n.fract() == 0.0 && n >= 0.0 && n <= u64::MAX as f64 {
        return rules.category_for(n as u64);
    }
    if n.fract() == 0.0 && n >= i64::MIN as f64 && n < 0.0 {
        return rules.category_for(PluralOperands::from(n as i64));
    }

    // ICU4X does not select from f64 (trailing zeros are CLDR-significant).
    icu_plurals::PluralCategory::Other
}

fn create_plural_rules(locale: &str) -> PluralRules {
    if let Some(rules) = try_for_locale(locale) {
        return rules;
    }

    let base = base_language(locale);
    if !base.eq_ignore_ascii_case(locale) {
        if let Some(rules) = try_for_locale(base) {
            return rules;
        }
    }

    try_for_locale(ENGLISH_LOCALE).expect("English CLDR cardinal rules are available")
}

fn try_for_locale(tag: &str) -> Option<PluralRules> {
    if !looks_like_bcp47(tag) {
        return None;
    }

    let locale = Locale::try_from_str(tag).ok()?;
    PluralRules::try_new_cardinal((&locale).into()).ok()
}

fn normalize_locale_tag(lang: &str) -> String {
    let trimmed = lang.trim().replace('_', "-");
    if trimmed.is_empty() {
        ENGLISH_LOCALE.to_string()
    } else {
        trimmed
    }
}

fn base_language(locale: &str) -> &str {
    match locale.find('-') {
        Some(index) if index > 0 => &locale[..index],
        _ => locale,
    }
}

/// Accepts language tags such as `en`, `pt-PT`, `zh-Hans-CN`.
/// Rejects free text so ICU4X does not silently use the root locale (always `other`).
fn looks_like_bcp47(locale: &str) -> bool {
    if locale.len() < 2 {
        return false;
    }

    let mut first_segment = true;
    let mut segment_length = 0usize;
    for c in locale.chars() {
        if c == '-' {
            if segment_length == 0 || (first_segment && segment_length < 2) {
                return false;
            }
            first_segment = false;
            segment_length = 0;
            continue;
        }

        let is_letter = c.is_ascii_alphabetic();
        let is_digit = c.is_ascii_digit();
        if first_segment {
            if !is_letter {
                return false;
            }
        } else if !is_letter && !is_digit {
            return false;
        }

        segment_length += 1;
        if segment_length > 8 {
            return false;
        }
    }

    if first_segment {
        segment_length >= 2
    } else {
        segment_length >= 1
    }
}

fn map_icu_category(category: icu_plurals::PluralCategory) -> PluralCategory {
    match category {
        icu_plurals::PluralCategory::Zero => PluralCategory::Zero,
        icu_plurals::PluralCategory::One => PluralCategory::One,
        icu_plurals::PluralCategory::Two => PluralCategory::Two,
        icu_plurals::PluralCategory::Few => PluralCategory::Few,
        icu_plurals::PluralCategory::Many => PluralCategory::Many,
        icu_plurals::PluralCategory::Other => PluralCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN_PLURAL_ROWS: &[(&str, f64, PluralCategory)] = &[
        ("ar", 0.0, PluralCategory::Zero),
        ("ar", 2.0, PluralCategory::Two),
        ("pl", 2.0, PluralCategory::Few),
        ("fr", 0.0, PluralCategory::One),
        ("en", 0.0, PluralCategory::Other),
        ("en", 1.0, PluralCategory::One),
    ];

    const ANTI_BUCKET_PLURAL_ROWS: &[(&str, f64, PluralCategory)] = &[
        ("he", 2.0, PluralCategory::Two),
        ("ja", 1.0, PluralCategory::Other),
        ("pt", 0.0, PluralCategory::One),
        ("pt-PT", 0.0, PluralCategory::Other),
        ("bg", 2.0, PluralCategory::Other),
        ("es", 0.0, PluralCategory::Other),
        ("fr-CA", 0.0, PluralCategory::One),
        ("ar_EG", 0.0, PluralCategory::Zero),
    ];

    #[test]
    fn resolve_category_golden_rows() {
        for (lang, n, expected) in GOLDEN_PLURAL_ROWS {
            assert_eq!(
                resolve_category(Some(*n), lang),
                *expected,
                "golden {lang} n={n}"
            );
        }
    }

    #[test]
    fn resolve_category_anti_bucket_rows() {
        for (lang, n, expected) in ANTI_BUCKET_PLURAL_ROWS {
            assert_eq!(
                resolve_category(Some(*n), lang),
                *expected,
                "anti-bucket {lang} n={n}"
            );
        }
    }

    #[test]
    fn resolve_category_inverts_one_other_heuristic() {
        assert_eq!(resolve_category(Some(0.0), "fr"), PluralCategory::One);
        assert_eq!(resolve_category(Some(2.0), "ru"), PluralCategory::Few);
    }

    #[test]
    fn resolve_category_null_number_returns_other() {
        assert_eq!(resolve_category(None, "ar"), PluralCategory::Other);
        assert_eq!(resolve_category(None, "en"), PluralCategory::Other);
    }

    #[test]
    fn resolve_category_blank_lang_falls_back_to_english() {
        for lang in ["", "   "] {
            assert_eq!(
                resolve_category(Some(1.0), lang),
                PluralCategory::One,
                "blank lang {lang:?} n=1"
            );
            assert_eq!(
                resolve_category(Some(0.0), lang),
                PluralCategory::Other,
                "blank lang {lang:?} n=0"
            );
        }
    }

    #[test]
    fn resolve_category_invalid_lang_falls_back_to_english() {
        assert_eq!(
            resolve_category(Some(0.0), "not a locale!!"),
            PluralCategory::Other
        );
        assert_eq!(
            resolve_category(Some(1.0), "not a locale!!"),
            PluralCategory::One
        );
    }

    #[test]
    fn resolve_category_underscore_locale() {
        assert_eq!(resolve_category(Some(1.0), "en_US"), PluralCategory::One);
        assert_eq!(resolve_category(Some(0.0), "ar_EG"), PluralCategory::Zero);
    }

    #[test]
    fn resolve_category_decimal_n() {
        assert_eq!(resolve_category(Some(1.5), "en"), PluralCategory::Other);
        assert_eq!(resolve_category(Some(1.5), "fr"), PluralCategory::Other);
    }

    #[test]
    fn looks_like_bcp47_accepts_common_tags() {
        assert!(looks_like_bcp47("en"));
        assert!(looks_like_bcp47("pt-PT"));
        assert!(looks_like_bcp47("zh-Hans-CN"));
        assert!(!looks_like_bcp47(""));
        assert!(!looks_like_bcp47("x"));
        assert!(!looks_like_bcp47("not a locale!!"));
    }
}
