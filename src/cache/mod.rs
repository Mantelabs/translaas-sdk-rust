//! In-memory caching abstractions for the Translaas SDK.
//!
//! Provides [`CacheMode`], byte-identical [`KeyBuilder`] keys, and a thread-safe
//! [`MemoryProvider`]. Client integration is implemented in a later release (#7).

#![warn(missing_docs)]

mod error;
mod key_builder;
mod memory;
mod memory_options;
mod mode;
mod provider;
mod ttl;

pub use error::CacheError;
pub use key_builder::{entry_key, group_key, locales_key, offline_key, project_key, KeyBuilder};
pub use memory::MemoryProvider;
pub use memory_options::{MemoryOptions, Statistics};
pub use mode::CacheMode;
pub use provider::Provider;
pub use ttl::Ttl;

#[cfg(test)]
mod golden_tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    use serde::Deserialize;

    use super::KeyBuilder;

    #[derive(Debug, Deserialize)]
    struct GoldenKeyCase {
        name: String,
        method: String,
        args: GoldenKeyArgs,
        want: String,
    }

    #[derive(Debug, Default, Deserialize)]
    struct GoldenKeyArgs {
        #[serde(default)]
        group: String,
        #[serde(default)]
        entry: String,
        #[serde(default)]
        lang: String,
        number: Option<f64>,
        #[serde(default)]
        parameters: HashMap<String, String>,
        #[serde(default)]
        project: String,
        #[serde(default)]
        channel: String,
        #[serde(default)]
        version: String,
        #[serde(default)]
        format: String,
        #[serde(default, rename = "includeContext")]
        include_context: Option<bool>,
    }

    fn fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/cache_keys.json")
    }

    #[test]
    fn key_builder_golden_vectors() {
        let data = fs::read_to_string(fixture_path()).expect("read golden file");
        let cases: Vec<GoldenKeyCase> = serde_json::from_str(&data).expect("parse golden file");
        let builder = KeyBuilder;

        for case in cases {
            let got = build_golden_key(&builder, &case);
            assert_eq!(got, case.want, "case {}", case.name);
        }
    }

    fn build_golden_key(builder: &KeyBuilder, case: &GoldenKeyCase) -> String {
        match case.method.as_str() {
            "entry" => builder.entry_key(
                &case.args.group,
                &case.args.entry,
                &case.args.lang,
                case.args.number,
                &case.args.parameters,
                &case.args.project,
                &case.args.channel,
                &case.args.version,
            ),
            "group" => builder.group_key(
                &case.args.project,
                &case.args.group,
                &case.args.lang,
                &case.args.format,
                &case.args.channel,
                &case.args.version,
                case.args.include_context,
            ),
            "project" => builder.project_key(
                &case.args.project,
                &case.args.lang,
                &case.args.format,
                &case.args.channel,
                &case.args.version,
                case.args.include_context,
            ),
            "locales" => {
                builder.locales_key(&case.args.project, &case.args.channel, &case.args.version)
            }
            "offline" => builder.offline_key(
                &case.args.project,
                &case.args.channel,
                &case.args.version,
                case.args.include_context,
            ),
            other => panic!("unknown method {other}"),
        }
    }
}
