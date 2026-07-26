//! Configuration for [`super::MemoryProvider`].

use std::sync::Arc;
use std::time::Instant;

/// Optional limits and instrumentation for [`super::MemoryProvider`].
#[derive(Clone, Default)]
pub struct MemoryOptions {
    /// When set, evicts the least recently used entry before inserting a new one.
    pub max_size: Option<usize>,
    /// Enables hit/miss counters exposed via [`super::MemoryProvider::statistics`].
    pub enable_statistics: bool,
    /// Injectable monotonic clock for deterministic tests.
    pub clock: Option<Arc<dyn Fn() -> Instant + Send + Sync>>,
}

impl MemoryOptions {
    /// Sets the maximum number of entries before LRU eviction.
    pub fn with_max_size(mut self, max_size: usize) -> Self {
        if max_size > 0 {
            self.max_size = Some(max_size);
        }
        self
    }

    /// Enables hit/miss statistics.
    pub fn with_statistics(mut self) -> Self {
        self.enable_statistics = true;
        self
    }

    /// Uses a custom monotonic clock (primarily for tests).
    pub fn with_clock(mut self, clock: Arc<dyn Fn() -> Instant + Send + Sync>) -> Self {
        self.clock = Some(clock);
        self
    }
}

/// Snapshot of optional cache counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Statistics {
    /// Successful cache hits.
    pub hits: u64,
    /// Cache misses (including expired entries).
    pub misses: u64,
    /// Current number of stored entries.
    pub size: usize,
}
