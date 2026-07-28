//! Language filter for sync operations.

/// Filters available locales by configured sync languages.
///
/// When `requested` is empty, returns a copy of all `available` locales.
/// Otherwise preserves `requested` order and includes only locales present in
/// `available` (case-sensitive match, Go parity).
pub fn filter_sync_languages(available: &[String], requested: &[String]) -> Vec<String> {
    if requested.is_empty() {
        return available.to_vec();
    }

    let available_set: std::collections::HashSet<&str> =
        available.iter().map(String::as_str).collect();

    requested
        .iter()
        .filter(|lang| available_set.contains(lang.as_str()))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn langs(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn empty_requested_returns_all_available() {
        let available = langs(&["en", "es", "fr"]);
        assert_eq!(
            filter_sync_languages(&available, &[]),
            langs(&["en", "es", "fr"])
        );
    }

    #[test]
    fn filters_to_configured_order() {
        let available = langs(&["en", "es", "fr"]);
        let requested = langs(&["es", "en"]);
        assert_eq!(
            filter_sync_languages(&available, &requested),
            langs(&["es", "en"])
        );
    }

    #[test]
    fn skips_unavailable_requested_languages() {
        let available = langs(&["en", "es"]);
        let requested = langs(&["en", "de", "es"]);
        assert_eq!(
            filter_sync_languages(&available, &requested),
            langs(&["en", "es"])
        );
    }

    #[test]
    fn case_sensitive_match() {
        let available = langs(&["en"]);
        let requested = langs(&["EN"]);
        assert!(filter_sync_languages(&available, &requested).is_empty());
    }
}
