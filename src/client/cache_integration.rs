//! In-memory cache wiring for live HTTP reads (parity with Go `cache_integration.go`).

use std::collections::HashMap;
use std::sync::Arc;

use crate::cache::{CacheMode, KeyBuilder, MemoryProvider, Provider};
use crate::models::RequestContext;

use super::Client;

/// Returns whether `op` participates in caching for `mode`.
pub(crate) fn should_cache(mode: CacheMode, op: &str) -> bool {
    match op {
        "entry" => mode == CacheMode::Entry,
        "group" => mode == CacheMode::Group || mode == CacheMode::Project,
        "project" => mode == CacheMode::Project,
        "locales" => mode != CacheMode::None,
        _ => false,
    }
}

impl Client {
    pub(crate) fn caching_enabled(&self, op: &str) -> bool {
        self.cache_provider.is_some() && should_cache(self.cache_mode, op)
    }

    pub(crate) fn try_cache_get<T: Clone + Send + Sync + 'static>(&self, key: &str) -> Option<T> {
        let provider = self.cache_provider.as_ref()?;
        match Provider::get(provider.as_ref(), key) {
            Ok(Some(value)) => Some(value),
            Ok(None) | Err(_) => None,
        }
    }

    pub(crate) fn cache_set<T: Clone + Send + Sync + 'static>(&self, key: &str, value: T) {
        if let Some(provider) = &self.cache_provider {
            let _ = Provider::set(provider.as_ref(), key, value, self.cache_ttl);
        }
    }

    pub(crate) fn has_cache_provider(&self) -> bool {
        self.cache_provider.is_some()
    }

    pub(crate) fn try_cache_get_string(&self, key: &str) -> Option<String> {
        self.try_cache_get(key)
    }

    pub(crate) fn cache_set_string(&self, key: &str, value: &str) {
        self.cache_set(key, value.to_string());
    }
}

pub(crate) fn resolve_entry_project<'a>(
    ctx: Option<&'a RequestContext>,
    default_project_id: Option<&'a str>,
) -> &'a str {
    if let Some(ctx) = ctx {
        if let Some(ref project) = ctx.project {
            let trimmed = project.trim();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
    }
    default_project_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

pub(crate) fn snapshot_channel(ctx: Option<&RequestContext>) -> &str {
    ctx.and_then(|c| c.channel.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

pub(crate) fn snapshot_version(ctx: Option<&RequestContext>) -> &str {
    ctx.and_then(|c| c.version.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
}

pub(crate) fn build_entry_cache_key(
    group: &str,
    entry: &str,
    lang: &str,
    number: Option<f64>,
    parameters: &HashMap<String, String>,
    ctx: Option<&RequestContext>,
    default_project_id: Option<&str>,
) -> String {
    KeyBuilder.entry_key(
        group,
        entry,
        lang,
        number,
        parameters,
        resolve_entry_project(ctx, default_project_id),
        snapshot_channel(ctx),
        snapshot_version(ctx),
    )
}

pub(crate) fn default_cache_provider(mode: CacheMode) -> Option<Arc<MemoryProvider>> {
    if mode == CacheMode::None {
        None
    } else {
        Some(Arc::new(MemoryProvider::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::should_cache;
    use crate::cache::CacheMode;

    #[test]
    fn should_cache_mode_matrix() {
        let cases = [
            (CacheMode::None, "entry", false),
            (CacheMode::None, "group", false),
            (CacheMode::None, "project", false),
            (CacheMode::None, "locales", false),
            (CacheMode::Entry, "entry", true),
            (CacheMode::Entry, "group", false),
            (CacheMode::Entry, "project", false),
            (CacheMode::Entry, "locales", true),
            (CacheMode::Group, "entry", false),
            (CacheMode::Group, "group", true),
            (CacheMode::Group, "project", false),
            (CacheMode::Group, "locales", true),
            (CacheMode::Project, "entry", false),
            (CacheMode::Project, "group", true),
            (CacheMode::Project, "project", true),
            (CacheMode::Project, "locales", true),
        ];

        for (mode, op, want) in cases {
            assert_eq!(should_cache(mode, op), want, "mode={mode} op={op}");
        }
    }
}
