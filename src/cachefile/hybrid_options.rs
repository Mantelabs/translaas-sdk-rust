//! Configuration for the hybrid L1-over-L2 offline cache.

use std::time::Duration;

const DEFAULT_HYBRID_MEMORY_EXPIRATION: Duration = Duration::from_secs(30 * 60);
const DEFAULT_HYBRID_MAX_ENTRIES: u32 = 1000;

/// Options for the in-memory L1 layer over a file-backed L2 [`super::Provider`].
#[derive(Debug, Clone)]
pub struct HybridOptions {
    /// When `false`, [`super::HybridProvider`] delegates to L2 only.
    pub enabled: bool,
    /// TTL for L1 entries. Zero uses the default (30 minutes).
    pub memory_expiration: Duration,
    /// LRU capacity per L1 partition (projects, groups, locales). Zero uses the default (1000).
    pub max_entries: u32,
}

impl Default for HybridOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_expiration: DEFAULT_HYBRID_MEMORY_EXPIRATION,
            max_entries: DEFAULT_HYBRID_MAX_ENTRIES,
        }
    }
}

impl HybridOptions {
    /// Returns options aligned with Go/.NET defaults (alias for [`Default::default`]).
    pub fn default_hybrid() -> Self {
        Self::default()
    }

    /// Disables the L1 memory layer.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Sets L1 entry TTL.
    pub fn with_memory_expiration(mut self, memory_expiration: Duration) -> Self {
        self.memory_expiration = memory_expiration;
        self
    }

    /// Sets the maximum number of entries per L1 partition.
    pub fn with_max_entries(mut self, max_entries: u32) -> Self {
        self.max_entries = max_entries;
        self
    }
}

pub(crate) fn normalize_hybrid_options(mut opts: HybridOptions) -> HybridOptions {
    if opts.memory_expiration.is_zero() {
        opts.memory_expiration = DEFAULT_HYBRID_MEMORY_EXPIRATION;
    }
    if opts.max_entries == 0 {
        opts.max_entries = DEFAULT_HYBRID_MAX_ENTRIES;
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hybrid_options_match_go() {
        let opts = HybridOptions::default();
        assert!(opts.enabled);
        assert_eq!(opts.memory_expiration, Duration::from_secs(30 * 60));
        assert_eq!(opts.max_entries, 1000);
    }

    #[test]
    fn normalize_applies_defaults_for_zero_values() {
        let opts = normalize_hybrid_options(HybridOptions {
            enabled: true,
            memory_expiration: Duration::ZERO,
            max_entries: 0,
        });
        assert_eq!(opts.memory_expiration, Duration::from_secs(30 * 60));
        assert_eq!(opts.max_entries, 1000);
    }
}
