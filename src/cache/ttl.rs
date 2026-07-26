//! Cache entry expiration configuration.

use std::time::Duration;

/// Absolute and/or sliding expiration applied on [`super::Provider::set`].
///
/// A zero duration in Go disables that expiration type; here use [`None`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ttl {
    /// Absolute lifetime from the time of insertion.
    pub absolute: Option<Duration>,
    /// Sliding window refreshed on each successful read.
    pub sliding: Option<Duration>,
}

impl Ttl {
    /// No expiration (entries live until removed, evicted, or cleared).
    pub fn none() -> Self {
        Self::default()
    }

    /// Absolute expiration only.
    pub fn absolute(duration: Duration) -> Self {
        Self {
            absolute: Some(duration),
            sliding: None,
        }
    }

    /// Sliding expiration only.
    pub fn sliding(duration: Duration) -> Self {
        Self {
            absolute: None,
            sliding: Some(duration),
        }
    }

    /// Both absolute and sliding expiration (either can expire an entry).
    pub fn both(absolute: Duration, sliding: Duration) -> Self {
        Self {
            absolute: Some(absolute),
            sliding: Some(sliding),
        }
    }
}
