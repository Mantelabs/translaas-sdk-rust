//! CLDR plural categories used in translation payloads.

/// CLDR plural category used in translation entry JSON objects.
///
/// Wire keys use lowercase CLDR names (`one`, `other`, …). [`PluralCategory::as_str`]
/// returns PascalCase names matching .NET enum names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    /// Zero plural form.
    Zero,
    /// One plural form.
    One,
    /// Two plural form.
    Two,
    /// Few plural form.
    Few,
    /// Many plural form.
    Many,
    /// Other plural form.
    Other,
}

impl PluralCategory {
    /// Returns the enum name used in .NET SDK plural maps (`Zero`, `One`, …).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zero => "Zero",
            Self::One => "One",
            Self::Two => "Two",
            Self::Few => "Few",
            Self::Many => "Many",
            Self::Other => "Other",
        }
    }
}

pub(crate) fn parse_plural_category(name: &str) -> Option<PluralCategory> {
    match name.trim().to_ascii_lowercase().as_str() {
        "zero" => Some(PluralCategory::Zero),
        "one" => Some(PluralCategory::One),
        "two" => Some(PluralCategory::Two),
        "few" => Some(PluralCategory::Few),
        "many" => Some(PluralCategory::Many),
        "other" => Some(PluralCategory::Other),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_plural_category_is_case_insensitive() {
        assert_eq!(parse_plural_category("ONE"), Some(PluralCategory::One));
        assert_eq!(
            parse_plural_category(" other "),
            Some(PluralCategory::Other)
        );
        assert_eq!(parse_plural_category("invalid"), None);
    }

    #[test]
    fn as_str_returns_pascal_case() {
        assert_eq!(PluralCategory::One.as_str(), "One");
        assert_eq!(PluralCategory::Other.as_str(), "Other");
    }
}
