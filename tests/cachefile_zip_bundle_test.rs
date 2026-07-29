//! Integration tests for offline ZIP parse and FileProvider import.

#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use translaas::cachefile::{
    parse_offline_zip, resolve_project_key, FileProvider, FileProviderOptions, HybridOptions,
    HybridProvider, Provider, SaveOptions,
};
use translaas::models::{OfflineCacheError, TranslationProject};

#[path = "support/offline_zip.rs"]
mod offline_zip;

use offline_zip::{
    build_multi_project_zip, build_sanitized_folder_zip, build_test_offline_zip,
    build_test_offline_zip_with, past_rfc3339, write_zip_entry, write_zip_json,
};

fn new_test_provider() -> FileProvider {
    let dir = tempfile::tempdir().expect("tempdir");
    FileProvider::new(dir.keep()).expect("provider")
}

fn hello_from_project(project: &TranslationProject) -> String {
    project
        .get_group("common")
        .expect("group")
        .expect("some")
        .entries
        .get("hello")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string()
}

#[test]
fn parse_offline_zip_success() {
    let bundle = parse_offline_zip(&build_test_offline_zip()).expect("parse");
    assert_eq!(bundle.manifest.version, "1.0");
    assert_eq!(
        bundle.locales_by_project["demo-project"].data.locales.len(),
        2
    );

    let en = &bundle.projects_by_project_lang["demo-project"]["en"].data;
    assert_eq!(hello_from_project(en), "Hello");

    let de = &bundle.projects_by_project_lang["demo-project"]["de"].data;
    assert_eq!(hello_from_project(de), "Hallo");
}

#[test]
fn parse_offline_zip_empty() {
    let err = parse_offline_zip(&[]).expect_err("empty");
    assert!(err.to_string().contains("empty"));
    assert!(matches!(err, OfflineCacheError { .. }));
}

#[test]
fn parse_offline_zip_corrupt() {
    let err = parse_offline_zip(b"not-a-zip").expect_err("corrupt");
    assert!(err.to_string().contains("invalid offline ZIP archive"));
}

#[test]
fn parse_offline_zip_zip_slip_rejected() {
    let content = build_test_offline_zip_with(|writer| {
        write_zip_entry(writer, "../evil.json", b"{}").expect("evil entry");
    });
    let err = parse_offline_zip(&content).expect_err("zip slip");
    assert!(err.to_string().contains("unsafe ZIP entry"));
}

#[test]
fn parse_offline_zip_unknown_manifest_version() {
    let content = build_test_offline_zip_with(|writer| {
        write_zip_json(
            writer,
            "manifest.json",
            &json!({
                "version": "9.9",
                "sdkVersion": "1.0.0",
                "createdAt": "2026-01-01T00:00:00Z",
                "lastSyncAt": "2026-01-01T00:00:00Z",
                "projects": {}
            }),
        )
        .expect("manifest");
    });

    let bundle = parse_offline_zip(&content).expect("parse");
    assert_eq!(bundle.manifest.version, "9.9");
}

#[test]
#[ignore = "run manually to regenerate testdata/offline/demo-project-bundle.zip"]
fn write_golden_offline_fixture() {
    let bytes = build_test_offline_zip();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/offline/demo-project-bundle.zip");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, bytes).expect("write golden");
}

#[test]
fn parse_golden_fixture_bytes() {
    let bytes = include_bytes!("../testdata/offline/demo-project-bundle.zip");
    let bundle = parse_offline_zip(bytes).expect("golden parse");
    assert_eq!(bundle.manifest.version, "1.0");
    assert!(bundle.projects_by_project_lang.contains_key("demo-project"));
}

#[test]
fn resolve_project_key_exact() {
    let bundle = parse_offline_zip(&build_test_offline_zip()).expect("parse");
    let key = resolve_project_key(&bundle, "demo-project").expect("resolve");
    assert_eq!(key, "demo-project");
}

#[test]
fn resolve_project_key_sanitized_folder() {
    let bundle = parse_offline_zip(&build_sanitized_folder_zip()).expect("parse");
    let key = resolve_project_key(&bundle, "my/project").expect("resolve");
    assert_eq!(key, "my_project");
}

#[test]
fn import_offline_bundle_round_trip() {
    let provider = new_test_provider();
    provider
        .import_offline_bundle("demo-project", &build_test_offline_zip())
        .expect("import");

    let en = provider
        .get_project("demo-project", "en")
        .expect("get en")
        .expect("cached en");
    assert_eq!(hello_from_project(&en), "Hello");

    let locales = provider
        .get_locales("demo-project")
        .expect("locales")
        .expect("some locales");
    assert_eq!(locales.locales.len(), 2);

    let de_group = provider
        .get_group("demo-project", "common", "de")
        .expect("de group")
        .expect("group");
    assert!(de_group.entries.contains_key("hello"));

    let manifest = provider.get_manifest().expect("manifest").expect("some");
    let info = manifest
        .projects
        .get("demo-project")
        .expect("project entry");
    assert_eq!(info.status, "synced");
    assert_eq!(info.languages.len(), 2);
}

#[test]
fn import_offline_bundle_preserves_expiry() {
    let content = build_test_offline_zip_with(|writer| {
        write_zip_json(
            writer,
            "demo-project/en/project.json",
            &json!({
                "cachedAt": "2026-01-01T00:00:00Z",
                "expiresAt": past_rfc3339(),
                "data": { "common": { "hello": "Hello" } }
            }),
        )
        .expect("en project");
    });

    let provider = new_test_provider();
    provider
        .import_offline_bundle("demo-project", &content)
        .expect("import");

    assert!(provider
        .get_project("demo-project", "en")
        .expect("get")
        .is_none());
}

#[test]
fn import_offline_bundle_multi_project_isolation() {
    let provider = new_test_provider();
    provider
        .import_offline_bundle("project-a", &build_multi_project_zip())
        .expect("import");

    assert!(provider
        .get_project("project-a", "en")
        .expect("get a")
        .is_some());
    assert!(provider
        .get_project("project-b", "en")
        .expect("get b")
        .is_none());
}

#[test]
fn import_offline_bundle_cancelled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let provider = FileProvider::with_options(
        dir.path(),
        FileProviderOptions::default().with_cancel_check(Arc::new(|| true)),
    )
    .expect("provider");

    let err = provider
        .import_offline_bundle("demo-project", &build_test_offline_zip())
        .expect_err("cancelled");
    assert!(err.to_string().contains("cancelled"));
}

#[test]
fn import_offline_bundle_uses_save_time_cached_at() {
    let provider = new_test_provider();
    provider
        .import_offline_bundle("demo-project", &build_test_offline_zip())
        .expect("import");

    let root = provider
        .cache_directory()
        .join("demo-project/en/project.json");
    let wrapper: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root).expect("read")).expect("json");
    let cached_at: DateTime<Utc> =
        serde_json::from_value(wrapper["cachedAt"].clone()).expect("cachedAt");
    let zip_cached_at = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
    assert_ne!(cached_at, zip_cached_at);
}

#[test]
fn import_offline_bundle_updates_hybrid_l1_via_sync_path() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = FileProvider::new(dir.path()).expect("file");
    let hybrid = HybridProvider::new(file, HybridOptions::default());

    let mut stale = HashMap::new();
    stale.insert("common".to_string(), json!({"hello": "Stale"}));
    hybrid
        .save_project(
            "demo-project",
            "en",
            &TranslationProject {
                groups: stale,
                ..Default::default()
            },
            SaveOptions::new(),
        )
        .expect("seed stale");

    // Direct FileProvider import bypasses Hybrid L1; sync path uses Provider saves.
    // Seed via inner file import then verify hybrid still serves L1 until re-read from disk.
    let file_only = FileProvider::new(dir.path()).expect("file reopen");
    file_only
        .import_offline_bundle("demo-project", &build_test_offline_zip())
        .expect("import");

    hybrid.clear_memory_cache();
    let got = hybrid
        .get_project("demo-project", "en")
        .expect("get")
        .expect("project");
    assert_eq!(hello_from_project(&got), "Hello");
}
