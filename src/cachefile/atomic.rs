//! Atomic JSON read/write helpers for on-disk cache files.

use std::fs;
use std::io;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub(crate) fn read_json_file(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(data) => Ok(Some(data)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

pub(crate) fn parse_json_file<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, String> {
    let Some(data) = read_json_file(path).map_err(|err| err.to_string())? else {
        return Ok(None);
    };

    serde_json::from_slice(&data).map(Some).map_err(|err| {
        format!(
            "decode {}: {err}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file")
        )
    })
}

pub(crate) fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create directory {}: {err}", parent.display()))?;
    }

    let payload = serde_json::to_vec_pretty(value).map_err(|err| format!("marshal JSON: {err}"))?;

    let tmp_path = {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file");
        path.with_file_name(format!("{file_name}.tmp"))
    };
    fs::write(&tmp_path, payload)
        .map_err(|err| format!("write temp file {}: {err}", tmp_path.display()))?;

    fs::rename(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        format!("rename {}: {err}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde::{Deserialize, Serialize};

    use super::{parse_json_file, write_json_atomic};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Sample {
        value: String,
    }

    #[test]
    fn write_json_atomic_produces_valid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sample.json");
        write_json_atomic(
            &path,
            &Sample {
                value: "hello".to_string(),
            },
        )
        .expect("write");

        let parsed: Sample = parse_json_file(&path).expect("parse").expect("some");
        assert_eq!(parsed.value, "hello");

        let raw = std::fs::read_to_string(path).expect("read");
        serde_json::from_str::<HashMap<String, String>>(&raw).expect("valid json");
    }
}
