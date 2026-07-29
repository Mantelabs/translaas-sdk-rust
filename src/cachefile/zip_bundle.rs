//! Offline ZIP bundle parsing and import helpers (HTTP spec §7.6).

use std::collections::HashMap;
use std::io::{Cursor, Read};

use chrono::{DateTime, Utc};

use crate::models::OfflineCacheError;

use super::paths::sanitize_path_segment;
use super::provider::{Provider, SaveOptions};
use super::types::{CacheManifest, CachedLocales, CachedProject};

/// Parsed offline ZIP contents keyed by archive path segments.
#[derive(Debug, Clone, PartialEq)]
pub struct OfflineBundle {
    /// Root manifest from `manifest.json` when present.
    pub manifest: CacheManifest,
    /// Locales wrappers keyed by project folder segment in the archive.
    pub locales_by_project: HashMap<String, CachedLocales>,
    /// Project wrappers keyed by project folder then language segment.
    pub projects_by_project_lang: HashMap<String, HashMap<String, CachedProject>>,
}

/// Reads an offline ZIP bundle (HTTP spec §7.6).
pub fn parse_offline_zip(content: &[u8]) -> Result<OfflineBundle, OfflineCacheError> {
    if content.is_empty() {
        return Err(zip_parse_err("offline ZIP content is empty", None));
    }

    let cursor = Cursor::new(content);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|err| zip_parse_err("invalid offline ZIP archive", Some(Box::new(err))))?;

    let mut bundle = OfflineBundle {
        manifest: CacheManifest::default(),
        locales_by_project: HashMap::new(),
        projects_by_project_lang: HashMap::new(),
    };

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| zip_parse_err("invalid offline ZIP archive", Some(Box::new(err))))?;
        let name = entry.name().to_string();

        if let Err(reason) = validate_zip_entry_name(&name) {
            return Err(zip_parse_err(
                format!("unsafe ZIP entry {name:?}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    reason,
                ))),
            ));
        }
        if name.ends_with('/') {
            continue;
        }

        let mut raw = Vec::new();
        entry.read_to_end(&mut raw).map_err(|err| {
            zip_parse_err(format!("read ZIP entry {name:?}"), Some(Box::new(err)))
        })?;

        if name == "manifest.json" {
            bundle.manifest = serde_json::from_slice(&raw)
                .map_err(|err| zip_parse_err("decode manifest.json", Some(Box::new(err))))?;
            continue;
        }

        let parts: Vec<&str> = name.split('/').collect();
        if parts.len() < 2 {
            continue;
        }

        let project_segment = parts[0].to_string();
        let file_name = parts[parts.len() - 1];

        match (file_name, parts.len()) {
            ("locales.json", 2) => {
                let wrapped: CachedLocales = serde_json::from_slice(&raw).map_err(|err| {
                    zip_parse_err(format!("decode {name:?}"), Some(Box::new(err)))
                })?;
                bundle.locales_by_project.insert(project_segment, wrapped);
            }
            ("project.json", 3) => {
                let lang_segment = parts[1].to_string();
                let wrapped: CachedProject = serde_json::from_slice(&raw).map_err(|err| {
                    zip_parse_err(format!("decode {name:?}"), Some(Box::new(err)))
                })?;
                bundle
                    .projects_by_project_lang
                    .entry(project_segment)
                    .or_default()
                    .insert(lang_segment, wrapped);
            }
            _ => {}
        }
    }

    Ok(bundle)
}

/// Maps a logical project id to the folder key used inside the bundle.
pub fn resolve_project_key(
    bundle: &OfflineBundle,
    project: &str,
) -> Result<String, OfflineCacheError> {
    let project = project.trim();
    if project.is_empty() {
        return Err(OfflineCacheError::new(
            "project must not be empty",
            None,
            None,
            None,
            None,
        ));
    }

    let sanitized = sanitize_path_segment(project)
        .map_err(|err| OfflineCacheError::new(err, None, None, None, None))?;

    if bundle.has_project_data(&sanitized) {
        return Ok(sanitized);
    }
    if project != sanitized && bundle.has_project_data(project) {
        return Ok(project.to_string());
    }

    if bundle.manifest.projects.contains_key(&sanitized) {
        return Ok(sanitized);
    }
    if project != sanitized && bundle.manifest.projects.contains_key(project) {
        return Ok(project.to_string());
    }

    Err(OfflineCacheError::new(
        format!("project {project:?} not found in offline bundle"),
        None,
        Some(project.to_string()),
        None,
        None,
    ))
}

impl OfflineBundle {
    fn has_project_data(&self, key: &str) -> bool {
        if self.locales_by_project.contains_key(key) {
            return true;
        }
        self.projects_by_project_lang
            .get(key)
            .is_some_and(|langs| !langs.is_empty())
    }
}

/// Persists one project from a parsed bundle through [`Provider::save_locales`] / [`save_project`].
pub(crate) fn apply_offline_bundle(
    cache: &impl Provider,
    project: &str,
    key: &str,
    bundle: &OfflineBundle,
) -> Result<(), OfflineCacheError> {
    let mut has_locales = false;

    if let Some(locales) = bundle.locales_by_project.get(key) {
        has_locales = true;
        let data = locales.data.clone();
        cache.save_locales(
            project,
            &data,
            save_options_from_wrapper(locales.expires_at),
        )?;
    }

    let projects_by_lang = bundle
        .projects_by_project_lang
        .get(key)
        .cloned()
        .unwrap_or_default();

    if !has_locales && projects_by_lang.is_empty() {
        return Err(OfflineCacheError::new(
            format!("no offline data found for project key {key:?}"),
            None,
            Some(project.to_string()),
            None,
            None,
        ));
    }

    for (lang, wrapped) in projects_by_lang {
        let data = wrapped.data.clone();
        cache.save_project(
            project,
            &lang,
            &data,
            save_options_from_wrapper(wrapped.expires_at),
        )?;
    }

    Ok(())
}

fn save_options_from_wrapper(expires_at: Option<DateTime<Utc>>) -> SaveOptions {
    SaveOptions::new().with_expires_at(expires_at)
}

fn validate_zip_entry_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("empty entry name".to_string());
    }
    if name.contains('\\') {
        return Err("backslash in entry name".to_string());
    }
    if name.contains("..") {
        return Err("parent traversal in entry name".to_string());
    }
    if name.starts_with('/') {
        return Err("absolute entry name".to_string());
    }

    let cleaned = clean_zip_path(name);
    if cleaned == ".." || cleaned.starts_with("../") {
        return Err("parent traversal in entry name".to_string());
    }

    Ok(())
}

fn clean_zip_path(name: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for segment in name.split('/') {
        match segment {
            "" | "." => continue,
            ".." => parts.push(".."),
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn zip_parse_err(
    message: impl Into<String>,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
) -> OfflineCacheError {
    OfflineCacheError::new(message, None, None, None, source)
}
