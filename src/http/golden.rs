#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;

    use serde::Deserialize;
    use serde_json::Value;
    use url::Url;

    use super::super::{
        append_query_values, build_url, inject_plural_n, merge_query_params, query_values,
    };
    use crate::models::{GetGroupTranslationsRequest, GetTranslationRequest};

    #[derive(Debug, Deserialize)]
    struct GoldenUrlCase {
        name: String,
        #[allow(dead_code)]
        source: String,
        #[serde(rename = "baseURL")]
        base_url: String,
        endpoint: String,
        #[serde(rename = "requestType")]
        request_type: String,
        request: Value,
        extra: Option<HashMap<String, String>>,
        #[serde(rename = "injectN")]
        inject_n: Option<f64>,
        #[serde(rename = "wantPath")]
        want_path: String,
        #[serde(rename = "wantQuery")]
        want_query: HashMap<String, String>,
    }

    fn testdata_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("testdata")
            .join(name)
    }

    #[test]
    fn golden_urls_match_go_vectors() {
        let data = fs::read_to_string(testdata_path("urls.json")).expect("read urls.json");
        let cases: Vec<GoldenUrlCase> =
            serde_json::from_str(&data).expect("parse urls.json golden cases");

        for case in cases {
            let raw_url = build_url(&case.base_url, &case.endpoint)
                .unwrap_or_else(|err| panic!("{} build_url: {err}", case.name));
            let mut url = Url::parse(&raw_url)
                .unwrap_or_else(|err| panic!("{} parse built url: {err}", case.name));

            append_golden_request(&mut url, &case.request_type, &case.request)
                .unwrap_or_else(|err| panic!("{} append request: {err}", case.name));

            let mut extra = case.extra.unwrap_or_default();
            if case.inject_n.is_some() && extra.is_empty() {
                extra = HashMap::new();
            }
            if let Some(n) = case.inject_n {
                inject_plural_n(&mut extra, Some(n));
            }
            if !extra.is_empty() {
                merge_query_params(&mut url, &extra);
            }

            assert_eq!(url.path(), case.want_path, "{}", case.name);

            let got = query_values(&url);
            assert_eq!(
                got.len(),
                case.want_query.len(),
                "{} query = {:?}, want {:?}",
                case.name,
                got,
                case.want_query
            );
            for (key, want) in &case.want_query {
                assert_eq!(got.get(key), Some(want), "{} key {key}", case.name);
            }
        }
    }

    fn append_golden_request(
        url: &mut Url,
        request_type: &str,
        request: &Value,
    ) -> Result<(), String> {
        match request_type {
            "none" => Ok(()),
            "translation" => {
                let req: GetTranslationRequest =
                    serde_json::from_value(request.clone()).map_err(|err| err.to_string())?;
                append_query_values(url, &req).map_err(|err| err.message)
            }
            "group" => {
                let req: GetGroupTranslationsRequest =
                    serde_json::from_value(request.clone()).map_err(|err| err.to_string())?;
                append_query_values(url, &req).map_err(|err| err.message)
            }
            other => Err(format!("unknown requestType {other}")),
        }
    }
}
