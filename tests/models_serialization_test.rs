//! Integration-style tests for the public `translaas::models` surface.

use translaas::models::{
    language_codes, parse_translaas_error, ApiError, ConfigurationError, GetTranslationRequest,
    NoLanguageError, OfflineCacheMissError, PluralCategory, RequestContext, TranslaasError,
    TranslationGroup, TranslationProject,
};

#[test]
fn public_exports_compile() {
    let _ = (
        TranslaasError {
            message: None,
            code: None,
        },
        ApiError {
            status_code: 400,
            code: None,
            message: None,
            response_content: None,
        },
        ConfigurationError {
            message: "x".to_string(),
        },
        OfflineCacheMissError::new_offline_cache_miss_error("p", "en", "g", "e"),
        NoLanguageError,
        RequestContext::default(),
        PluralCategory::One,
        language_codes::EN,
    );
    let _req = GetTranslationRequest {
        group: None,
        entry: None,
        lang: None,
        n: None,
        project: None,
        channel: None,
        version: None,
    };
}

#[test]
fn translation_group_dual_shape_round_trip() {
    let flat = include_str!("../testdata/translation_group_flat_simple.json");
    let full = include_str!("../testdata/translation_group_full_api.json");

    for json in [flat, full] {
        let group: TranslationGroup = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_string(&group).unwrap();
        let again: TranslationGroup = serde_json::from_str(&serialized).unwrap();
        assert_eq!(again.entries.len(), group.entries.len());
    }
}

#[test]
fn request_context_reset_preserves_request_fields() {
    let mut ctx = RequestContext {
        channel: Some("stable".to_string()),
        version: Some("1".to_string()),
        project: Some("proj".to_string()),
        include_context: Some(true),
        if_none_match: Some("req-etag".to_string()),
        response_etag: Some("res-etag".to_string()),
        not_modified: true,
    };
    ctx.reset();
    assert_eq!(ctx.channel.as_deref(), Some("stable"));
    assert!(ctx.response_etag.is_none());
    assert!(!ctx.not_modified);
}

#[test]
fn parse_translaas_error_empty_body() {
    assert!(parse_translaas_error(b"").unwrap().is_none());
}

#[test]
fn translation_project_get_group_from_fixture() {
    let json = include_str!("../testdata/translation_project_flat.json");
    let project: TranslationProject = serde_json::from_str(json).unwrap();
    let ui = project.get_group("ui").unwrap().unwrap();
    assert_eq!(ui.get_value("button.save"), Some("Save"));
}
