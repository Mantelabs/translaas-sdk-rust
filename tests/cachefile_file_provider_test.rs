//! Integration tests for `translaas::cachefile::FileProvider`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::json;
use translaas::cachefile::{
    CacheManifest, FileProvider, FileProviderOptions, Provider, SaveOptions, MANIFEST_VERSION,
};
use translaas::models::{OfflineCacheError, ProjectLocales, TranslationProject};

fn new_test_provider() -> FileProvider {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.keep();
    FileProvider::new(path).expect("new provider")
}

fn sample_project() -> TranslationProject {
    let mut groups = HashMap::new();
    groups.insert(
        "common".to_string(),
        json!({"hello": "Hello", "bye": "Bye"}),
    );
    TranslationProject {
        groups,
        ..Default::default()
    }
}

#[test]
fn file_provider_round_trip_project() {
    let provider = new_test_provider();
    let project = sample_project();

    provider
        .save_project("demo-project", "en", &project, SaveOptions::new())
        .expect("save");

    let got = provider
        .get_project("demo-project", "en")
        .expect("get")
        .expect("cached project");

    let group = got.get_group("common").expect("group").expect("some group");
    let hello = group
        .entries
        .get("hello")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(hello, "Hello");
}

#[test]
fn file_provider_get_group() {
    let provider = new_test_provider();
    provider
        .save_project("demo-project", "en", &sample_project(), SaveOptions::new())
        .expect("save");

    let group = provider
        .get_group("demo-project", "common", "en")
        .expect("get group")
        .expect("group");
    assert!(group.entries.contains_key("hello"));

    let missing = provider
        .get_group("demo-project", "missing", "en")
        .expect("missing group");
    assert!(missing.is_none());
}

#[test]
fn file_provider_round_trip_locales() {
    let provider = new_test_provider();
    let locales = ProjectLocales {
        project: Some("demo-project".to_string()),
        locales: vec!["en".to_string(), "de".to_string()],
        last_modified_utc: None,
    };

    provider
        .save_locales("demo-project", &locales, SaveOptions::new())
        .expect("save");

    let got = provider
        .get_locales("demo-project")
        .expect("get")
        .expect("locales");
    assert_eq!(got.locales, vec!["en", "de"]);
}

#[test]
fn file_provider_expired_entry() {
    let provider = new_test_provider();
    let past = Utc::now() - Duration::hours(1);

    provider
        .save_project(
            "demo-project",
            "en",
            &sample_project(),
            SaveOptions::new().with_expires_at(Some(past)),
        )
        .expect("save");

    assert!(provider
        .get_project("demo-project", "en")
        .expect("get")
        .is_none());
    assert!(!provider.is_cached("demo-project", "en").expect("is cached"));
}

#[test]
fn file_provider_missing_files() {
    let provider = new_test_provider();
    assert!(provider
        .get_project("missing", "en")
        .expect("get")
        .is_none());
}

#[test]
fn file_provider_corrupt_json() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("demo-project").join("en")).expect("mkdir");
    std::fs::write(root.join("demo-project/en/project.json"), "{not-json").expect("write");

    let provider = FileProvider::new(&root).expect("new");
    let err = provider
        .get_project("demo-project", "en")
        .expect_err("corrupt json");
    assert!(err.to_string().contains("read project cache"));
    assert!(matches!(err, OfflineCacheError { .. }));
}

#[test]
fn file_provider_manifest_after_save() {
    let provider = new_test_provider();
    provider
        .save_project("demo-project", "en", &sample_project(), SaveOptions::new())
        .expect("save");

    let manifest = provider.get_manifest().expect("manifest").expect("some");
    assert_eq!(manifest.version, MANIFEST_VERSION);
    let info = manifest
        .projects
        .get("demo-project")
        .expect("project entry");
    assert_eq!(info.languages, vec!["en"]);
}

#[test]
fn file_provider_clear() {
    let provider = new_test_provider();
    provider
        .save_project("demo-project", "en", &sample_project(), SaveOptions::new())
        .expect("save");

    provider.clear().expect("clear");
    assert!(provider
        .get_project("demo-project", "en")
        .expect("get")
        .is_none());
}

#[test]
fn file_provider_cancelled_operation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cancelled = Arc::new(AtomicBool::new(true));
    let flag = Arc::clone(&cancelled);
    let provider = FileProvider::with_options(
        dir.path(),
        FileProviderOptions::default()
            .with_cancel_check(Arc::new(move || flag.load(Ordering::SeqCst))),
    )
    .expect("new");

    let err = provider
        .save_project("demo-project", "en", &sample_project(), SaveOptions::new())
        .expect_err("cancelled");
    assert!(err.to_string().contains("operation cancelled"));
}

#[test]
fn file_provider_atomic_write_replaces_existing() {
    let provider = new_test_provider();
    provider
        .save_project("demo-project", "en", &sample_project(), SaveOptions::new())
        .expect("first save");

    let mut updated = TranslationProject::default();
    updated
        .groups
        .insert("common".to_string(), json!({"hello": "Updated"}));

    provider
        .save_project("demo-project", "en", &updated, SaveOptions::new())
        .expect("second save");

    let got = provider
        .get_project("demo-project", "en")
        .expect("get")
        .expect("project");
    let group = got.get_group("common").expect("group").expect("some");
    let hello = group
        .entries
        .get("hello")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    assert_eq!(hello, "Updated");

    let raw = std::fs::read_to_string(
        provider
            .cache_directory()
            .join("demo-project/en/project.json"),
    )
    .expect("read file");
    serde_json::from_str::<serde_json::Value>(&raw).expect("valid json");
}

#[test]
fn file_provider_locales_fallback_manifest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let provider = FileProvider::new(root).expect("new");

    let manifest = CacheManifest {
        version: MANIFEST_VERSION.to_string(),
        sdk_version: "1.0.0".to_string(),
        created_at: Utc::now(),
        last_sync_at: Utc::now(),
        projects: HashMap::from([(
            "demo-project".to_string(),
            translaas::cachefile::ProjectCacheInfo {
                languages: vec!["en".to_string(), "fr".to_string()],
                last_sync_at: Utc::now(),
                status: "synced".to_string(),
            },
        )]),
    };

    std::fs::create_dir_all(root).expect("mkdir");
    std::fs::write(
        root.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("marshal"),
    )
    .expect("write manifest");

    let got = provider
        .get_locales("demo-project")
        .expect("get")
        .expect("locales");
    assert_eq!(got.locales, vec!["en", "fr"]);
}

#[test]
fn file_provider_locales_fallback_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let provider = FileProvider::new(root).expect("new");

    let project_dir = root.join("demo-project/de");
    std::fs::create_dir_all(&project_dir).expect("mkdir");
    let wrapped = translaas::cachefile::CachedProject {
        cached_at: Utc::now(),
        expires_at: None,
        data: sample_project(),
    };
    std::fs::write(
        project_dir.join("project.json"),
        serde_json::to_vec_pretty(&wrapped).expect("marshal"),
    )
    .expect("write project");

    let got = provider
        .get_locales("demo-project")
        .expect("get")
        .expect("locales");
    assert_eq!(got.locales, vec!["de"]);
}
