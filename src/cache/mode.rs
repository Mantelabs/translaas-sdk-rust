//! In-memory cache mode controlling which client operations are cached.

use std::fmt;

/// Controls which client operations participate in in-memory caching.
///
/// Wiring lives in the client package: [`CacheMode::None`] disables caching;
/// [`CacheMode::Entry`] caches single-entry lookups; [`CacheMode::Group`] caches
/// group payloads; [`CacheMode::Project`] caches full project payloads.
/// Project locales are cached whenever the mode is not [`CacheMode::None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CacheMode {
    /// Disables in-memory caching.
    #[default]
    None,
    /// Caches individual entry (`get_entry`) results.
    Entry,
    /// Caches group (`get_group`) payloads.
    Group,
    /// Caches project (`get_project`) payloads.
    Project,
}

impl fmt::Display for CacheMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CacheMode::None => "None",
            CacheMode::Entry => "Entry",
            CacheMode::Group => "Group",
            CacheMode::Project => "Project",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CacheMode;

    #[test]
    fn display_matches_go_mode_string() {
        let cases = [
            (CacheMode::None, "None"),
            (CacheMode::Entry, "Entry"),
            (CacheMode::Group, "Group"),
            (CacheMode::Project, "Project"),
        ];
        for (mode, want) in cases {
            assert_eq!(mode.to_string(), want);
        }
    }
}
