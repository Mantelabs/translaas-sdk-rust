//! Shared offline ZIP builders for cachefile integration tests.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Cursor, Write};

use chrono::Utc;
use serde_json::{json, Value};
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

type ZipWriterCursor = ZipWriter<Cursor<Vec<u8>>>;
type ZipEntries = BTreeMap<String, Vec<u8>>;

fn minimal_manifest(projects: Value) -> Value {
    json!({
        "version": "1.0",
        "sdkVersion": "1.0.0",
        "createdAt": "2026-01-01T00:00:00Z",
        "lastSyncAt": "2026-01-01T00:00:00Z",
        "projects": projects,
    })
}

fn put_zip_json(entries: &mut ZipEntries, name: &str, value: &Value) {
    let payload = serde_json::to_vec(value).expect("marshal json");
    entries.insert(name.to_string(), payload);
}

fn finish_zip_entries(entries: ZipEntries) -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);
    for (name, payload) in entries {
        write_zip_entry(&mut writer, &name, &payload).expect("zip entry");
    }
    writer.finish().expect("finish zip").into_inner()
}

pub fn build_test_offline_zip() -> Vec<u8> {
    build_test_offline_zip_with(|_| {})
}

/// Builds the standard demo offline ZIP, then lets `mutate` insert or replace entries by path.
///
/// Replacements are required on zip 8+, which rejects duplicate filenames in [`ZipWriter`].
pub fn build_test_offline_zip_with(mut mutate: impl FnMut(&mut ZipEntries)) -> Vec<u8> {
    let mut entries = ZipEntries::new();

    let manifest = json!({
        "version": "1.0",
        "sdkVersion": "1.0.0",
        "createdAt": "2026-01-01T00:00:00Z",
        "lastSyncAt": "2026-01-01T00:00:00Z",
        "projects": {
            "demo-project": {
                "languages": ["en", "de"],
                "lastSyncAt": "2026-01-01T00:00:00Z",
                "status": "synced"
            }
        }
    });
    put_zip_json(&mut entries, "manifest.json", &manifest);

    let locales_wrapper = json!({
        "cachedAt": "2026-01-01T00:00:00Z",
        "data": { "locales": ["en", "de"] }
    });
    put_zip_json(&mut entries, "demo-project/locales.json", &locales_wrapper);

    let en_project = json!({
        "cachedAt": "2026-01-01T00:00:00Z",
        "data": { "common": { "hello": "Hello" } }
    });
    put_zip_json(&mut entries, "demo-project/en/project.json", &en_project);

    let de_project = json!({
        "cachedAt": "2026-01-01T00:00:00Z",
        "data": { "common": { "hello": "Hallo" } }
    });
    put_zip_json(&mut entries, "demo-project/de/project.json", &de_project);

    mutate(&mut entries);
    finish_zip_entries(entries)
}

pub fn write_zip_json(
    writer: &mut ZipWriterCursor,
    name: &str,
    value: &Value,
) -> Result<(), zip::result::ZipError> {
    let payload = serde_json::to_vec(value).expect("marshal json");
    write_zip_entry(writer, name, &payload)
}

pub fn write_zip_entry(
    writer: &mut ZipWriterCursor,
    name: &str,
    payload: &[u8],
) -> Result<(), zip::result::ZipError> {
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file(name, options)?;
    writer.write_all(payload)?;
    Ok(())
}

pub fn build_sanitized_folder_zip() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    write_zip_json(&mut writer, "manifest.json", &minimal_manifest(json!({}))).expect("manifest");
    write_zip_json(
        &mut writer,
        "my_project/locales.json",
        &json!({
            "cachedAt": "2026-01-01T00:00:00Z",
            "data": { "locales": ["en"] }
        }),
    )
    .expect("locales");
    write_zip_json(
        &mut writer,
        "my_project/en/project.json",
        &json!({
            "cachedAt": "2026-01-01T00:00:00Z",
            "data": { "common": { "hello": "Hi" } }
        }),
    )
    .expect("project");

    writer.finish().expect("finish").into_inner()
}

pub fn build_multi_project_zip() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut writer = ZipWriter::new(cursor);

    write_zip_json(&mut writer, "manifest.json", &minimal_manifest(json!({}))).expect("manifest");
    write_zip_json(
        &mut writer,
        "project-a/en/project.json",
        &json!({
            "cachedAt": "2026-01-01T00:00:00Z",
            "data": { "common": { "hello": "A" } }
        }),
    )
    .expect("project-a");
    write_zip_json(
        &mut writer,
        "project-b/en/project.json",
        &json!({
            "cachedAt": "2026-01-01T00:00:00Z",
            "data": { "common": { "hello": "B" } }
        }),
    )
    .expect("project-b");

    writer.finish().expect("finish").into_inner()
}

pub fn past_rfc3339() -> String {
    (Utc::now() - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}
