//! File-backed offline cache provider.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use crate::models::{OfflineCacheError, ProjectLocales, TranslationGroup, TranslationProject};

use super::atomic::{parse_json_file, write_json_atomic};
use super::paths::{
    check_cancelled, is_expired, offline_cache_err, resolve_absolute_path, sanitize_path_segment,
};
use super::provider::{Provider, SaveOptions};
use super::types::{
    CacheManifest, CachedLocales, CachedProject, ProjectCacheInfo, DEFAULT_SDK_VERSION,
    MANIFEST_VERSION,
};

type NowFn = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;
type CancelFn = Arc<dyn Fn() -> bool + Send + Sync>;

/// Persists offline translation payloads as JSON on disk.
pub struct FileProvider {
    dir: PathBuf,
    lock: RwLock<()>,
    now: NowFn,
    cancel_check: CancelFn,
}

impl FileProvider {
    /// Creates a file-backed offline cache at `cache_directory`.
    ///
    /// Relative paths resolve against the process working directory.
    pub fn new(cache_directory: impl AsRef<Path>) -> Result<Self, OfflineCacheError> {
        Self::with_options(cache_directory, FileProviderOptions::default())
    }

    /// Creates a provider with injectable clock and cancellation hooks (for tests).
    #[doc(hidden)]
    pub fn with_options(
        cache_directory: impl AsRef<Path>,
        options: FileProviderOptions,
    ) -> Result<Self, OfflineCacheError> {
        let dir = resolve_absolute_path(cache_directory.as_ref())?;
        Ok(Self {
            dir,
            lock: RwLock::new(()),
            now: options.now.unwrap_or_else(|| Arc::new(Utc::now)),
            cancel_check: options.cancel_check.unwrap_or_else(|| Arc::new(|| false)),
        })
    }

    /// Returns the absolute cache root path.
    pub fn cache_directory(&self) -> &Path {
        &self.dir
    }

    fn now(&self) -> DateTime<Utc> {
        (self.now)()
    }

    fn sanitized_project_dir(&self, project: &str) -> Result<(PathBuf, String), OfflineCacheError> {
        let safe = sanitize_path_segment(project).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                "",
                format!("invalid project id: {err}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    err,
                ))),
            )
        })?;
        Ok((self.dir.join(&safe), safe))
    }

    fn project_file(&self, project_dir: &Path, lang: &str) -> Result<PathBuf, OfflineCacheError> {
        let safe_lang = sanitize_path_segment(lang).map_err(|err| {
            offline_cache_err(
                &self.dir,
                "",
                lang,
                format!("invalid language: {lang}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    err,
                ))),
            )
        })?;
        Ok(project_dir.join(safe_lang).join("project.json"))
    }

    fn locales_file(&self, project_dir: &Path) -> PathBuf {
        project_dir.join("locales.json")
    }

    fn manifest_file(&self) -> PathBuf {
        self.dir.join("manifest.json")
    }

    fn get_project_locked(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        let (project_dir, _) = self.sanitized_project_dir(project)?;
        let project_path = self.project_file(&project_dir, lang)?;

        let wrapped = parse_json_file::<CachedProject>(&project_path).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                lang,
                format!("read project cache: {err}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err,
                ))),
            )
        })?;

        let Some(wrapped) = wrapped else {
            return Ok(None);
        };

        if is_expired(wrapped.expires_at, self.now()) {
            return Ok(None);
        }

        Ok(Some(wrapped.data))
    }

    fn get_manifest_locked(&self) -> Result<Option<CacheManifest>, OfflineCacheError> {
        parse_json_file::<CacheManifest>(&self.manifest_file()).map_err(|err| {
            offline_cache_err(
                &self.dir,
                "",
                "",
                format!("read manifest: {err}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err,
                ))),
            )
        })
    }

    fn update_manifest_locked(
        &self,
        update: &mut dyn FnMut(&mut CacheManifest) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        let mut manifest = self
            .get_manifest_locked()?
            .unwrap_or_else(|| CacheManifest {
                version: MANIFEST_VERSION.to_string(),
                sdk_version: DEFAULT_SDK_VERSION.to_string(),
                created_at: self.now(),
                last_sync_at: self.now(),
                projects: Default::default(),
            });

        update(&mut manifest)?;

        manifest.last_sync_at = self.now();
        write_json_atomic(&self.manifest_file(), &manifest).map_err(|err| {
            offline_cache_err(
                &self.dir,
                "",
                "",
                format!("write manifest: {err}"),
                Some(Box::new(std::io::Error::other(err))),
            )
        })
    }

    fn record_project_language_locked(
        &self,
        sanitized_project: &str,
        lang: &str,
    ) -> Result<(), OfflineCacheError> {
        self.update_manifest_locked(&mut |manifest| {
            let info = manifest
                .projects
                .entry(sanitized_project.to_string())
                .or_insert_with(|| ProjectCacheInfo {
                    languages: Vec::new(),
                    last_sync_at: self.now(),
                    status: "synced".to_string(),
                });
            info.languages = append_language(std::mem::take(&mut info.languages), lang);
            info.last_sync_at = self.now();
            info.status = "synced".to_string();
            Ok(())
        })
    }

    fn record_project_locales_locked(
        &self,
        sanitized_project: &str,
        locales: &[String],
    ) -> Result<(), OfflineCacheError> {
        self.update_manifest_locked(&mut |manifest| {
            let info = manifest
                .projects
                .entry(sanitized_project.to_string())
                .or_insert_with(|| ProjectCacheInfo {
                    languages: Vec::new(),
                    last_sync_at: self.now(),
                    status: "synced".to_string(),
                });
            for lang in normalize_locales(locales) {
                info.languages = append_language(std::mem::take(&mut info.languages), &lang);
            }
            info.last_sync_at = self.now();
            info.status = "synced".to_string();
            Ok(())
        })
    }

    fn get_locales_locked(
        &self,
        project: &str,
    ) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        let (project_dir, sanitized_project) = self.sanitized_project_dir(project)?;
        let locales_path = self.locales_file(&project_dir);

        let wrapped = parse_json_file::<CachedLocales>(&locales_path).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                "",
                format!("read locales cache: {err}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err,
                ))),
            )
        })?;

        if let Some(wrapped) = wrapped {
            if !is_expired(wrapped.expires_at, self.now()) {
                let mut out = wrapped.data;
                if out.project.as_deref().unwrap_or("").is_empty() {
                    out.project = Some(project.to_string());
                }
                return Ok(Some(out));
            }
        }

        let manifest = self.get_manifest_locked()?;
        if let Some(locales) = locales_from_manifest(manifest.as_ref(), &sanitized_project) {
            return Ok(Some(ProjectLocales {
                project: Some(project.to_string()),
                locales,
                last_modified_utc: None,
            }));
        }

        let scanned = self.scan_cached_locale_directories(&project_dir)?;
        if scanned.is_empty() {
            return Ok(None);
        }

        Ok(Some(ProjectLocales {
            project: Some(project.to_string()),
            locales: scanned,
            last_modified_utc: None,
        }))
    }

    fn scan_cached_locale_directories(
        &self,
        project_dir: &Path,
    ) -> Result<Vec<String>, OfflineCacheError> {
        let entries = match fs::read_dir(project_dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => {
                return Err(offline_cache_err(
                    &self.dir,
                    "",
                    "",
                    format!("scan locale directories: {err}"),
                    Some(Box::new(err)),
                ));
            }
        };

        let mut locales = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| {
                offline_cache_err(
                    &self.dir,
                    "",
                    "",
                    format!("scan locale directories: {err}"),
                    Some(Box::new(err)),
                )
            })?;
            if !entry
                .file_type()
                .map_err(|err| {
                    offline_cache_err(
                        &self.dir,
                        "",
                        "",
                        format!("scan locale directories: {err}"),
                        Some(Box::new(err)),
                    )
                })?
                .is_dir()
            {
                continue;
            }

            let lang = entry.file_name().to_string_lossy().into_owned();
            let project_path = entry.path().join("project.json");
            match fs::metadata(&project_path) {
                Ok(_) => locales.push(lang),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(offline_cache_err(
                        &self.dir,
                        "",
                        "",
                        format!("scan locale directories: {err}"),
                        Some(Box::new(err)),
                    ));
                }
            }
        }

        Ok(locales)
    }
}

/// Injectable hooks for [`FileProvider::with_options`].
#[derive(Default)]
pub struct FileProviderOptions {
    now: Option<NowFn>,
    cancel_check: Option<CancelFn>,
}

impl FileProviderOptions {
    /// Sets the clock used for expiry checks and wrapper timestamps.
    pub fn with_now(mut self, now: NowFn) -> Self {
        self.now = Some(now);
        self
    }

    /// Sets a cancellation predicate checked before each operation.
    pub fn with_cancel_check(mut self, cancel_check: CancelFn) -> Self {
        self.cancel_check = Some(cancel_check);
        self
    }
}

impl Provider for FileProvider {
    fn get_project(
        &self,
        project: &str,
        lang: &str,
    ) -> Result<Option<TranslationProject>, OfflineCacheError> {
        check_cancelled(&self.dir, project, lang, (self.cancel_check)())?;
        let _guard = self.lock.read().map_err(|_| poisoned_lock_err(&self.dir))?;
        self.get_project_locked(project, lang)
    }

    fn save_project(
        &self,
        project: &str,
        lang: &str,
        data: &TranslationProject,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        check_cancelled(&self.dir, project, lang, (self.cancel_check)())?;

        let _guard = self
            .lock
            .write()
            .map_err(|_| poisoned_lock_err(&self.dir))?;

        let (project_dir, sanitized_project) = self.sanitized_project_dir(project)?;
        let project_path = self.project_file(&project_dir, lang)?;

        let wrapped = CachedProject {
            cached_at: options.cached_at,
            expires_at: options.expires_at,
            data: data.clone(),
        };

        write_json_atomic(&project_path, &wrapped).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                lang,
                format!("write project cache: {err}"),
                Some(Box::new(std::io::Error::other(err))),
            )
        })?;

        self.record_project_language_locked(&sanitized_project, lang)
    }

    fn get_group(
        &self,
        project: &str,
        group: &str,
        lang: &str,
    ) -> Result<Option<TranslationGroup>, OfflineCacheError> {
        let Some(project_data) = self.get_project(project, lang)? else {
            return Ok(None);
        };

        project_data.get_group(group).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                lang,
                format!("read group {group}: {err}"),
                Some(Box::new(err)),
            )
        })
    }

    fn get_locales(&self, project: &str) -> Result<Option<ProjectLocales>, OfflineCacheError> {
        check_cancelled(&self.dir, project, "", (self.cancel_check)())?;
        let _guard = self.lock.read().map_err(|_| poisoned_lock_err(&self.dir))?;
        self.get_locales_locked(project)
    }

    fn save_locales(
        &self,
        project: &str,
        data: &ProjectLocales,
        options: SaveOptions,
    ) -> Result<(), OfflineCacheError> {
        check_cancelled(&self.dir, project, "", (self.cancel_check)())?;

        let _guard = self
            .lock
            .write()
            .map_err(|_| poisoned_lock_err(&self.dir))?;

        let (project_dir, sanitized_project) = self.sanitized_project_dir(project)?;

        let mut payload = data.clone();
        if payload.project.as_deref().unwrap_or("").is_empty() {
            payload.project = Some(project.to_string());
        }

        let wrapped = CachedLocales {
            cached_at: options.cached_at,
            expires_at: options.expires_at,
            data: payload.clone(),
        };

        write_json_atomic(&self.locales_file(&project_dir), &wrapped).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                "",
                format!("write locales cache: {err}"),
                Some(Box::new(std::io::Error::other(err))),
            )
        })?;

        self.record_project_locales_locked(&sanitized_project, &payload.locales)
    }

    fn get_manifest(&self) -> Result<Option<CacheManifest>, OfflineCacheError> {
        check_cancelled(&self.dir, "", "", (self.cancel_check)())?;
        let _guard = self.lock.read().map_err(|_| poisoned_lock_err(&self.dir))?;
        self.get_manifest_locked()
    }

    fn update_manifest(
        &self,
        update: &mut dyn FnMut(&mut CacheManifest) -> Result<(), OfflineCacheError>,
    ) -> Result<(), OfflineCacheError> {
        check_cancelled(&self.dir, "", "", (self.cancel_check)())?;
        let _guard = self
            .lock
            .write()
            .map_err(|_| poisoned_lock_err(&self.dir))?;
        self.update_manifest_locked(update)
    }

    fn is_cached(&self, project: &str, lang: &str) -> Result<bool, OfflineCacheError> {
        check_cancelled(&self.dir, project, lang, (self.cancel_check)())?;
        let _guard = self.lock.read().map_err(|_| poisoned_lock_err(&self.dir))?;

        let (project_dir, _) = self.sanitized_project_dir(project)?;
        let project_path = self.project_file(&project_dir, lang)?;

        let wrapped = parse_json_file::<CachedProject>(&project_path).map_err(|err| {
            offline_cache_err(
                &self.dir,
                project,
                lang,
                format!("read project cache: {err}"),
                Some(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    err,
                ))),
            )
        })?;

        Ok(wrapped.is_some_and(|entry| !is_expired(entry.expires_at, self.now())))
    }

    fn clear(&self) -> Result<(), OfflineCacheError> {
        check_cancelled(&self.dir, "", "", (self.cancel_check)())?;
        let _guard = self
            .lock
            .write()
            .map_err(|_| poisoned_lock_err(&self.dir))?;

        match fs::remove_dir_all(&self.dir) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(offline_cache_err(
                &self.dir,
                "",
                "",
                format!("clear cache directory: {err}"),
                Some(Box::new(err)),
            )),
        }
    }
}

fn poisoned_lock_err(dir: &Path) -> OfflineCacheError {
    offline_cache_err(dir, "", "", "cache lock poisoned", None)
}

fn contains_language(languages: &[String], lang: &str) -> bool {
    languages.iter().any(|existing| existing == lang)
}

fn append_language(languages: Vec<String>, lang: &str) -> Vec<String> {
    if contains_language(&languages, lang) {
        return languages;
    }
    let mut out = languages;
    out.push(lang.to_string());
    out
}

fn normalize_locales(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !contains_language(&out, trimmed) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn locales_from_manifest(
    manifest: Option<&CacheManifest>,
    sanitized_project: &str,
) -> Option<Vec<String>> {
    let manifest = manifest?;
    let info = manifest.projects.get(sanitized_project)?;
    let locales = normalize_locales(&info.languages);
    if locales.is_empty() {
        None
    } else {
        Some(locales)
    }
}
