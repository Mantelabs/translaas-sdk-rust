//! Foundation smoke tests — prove the crate links under default features.

#[test]
fn crate_links() {
    assert_eq!(env!("CARGO_PKG_NAME"), "translaas");
}

#[test]
fn crate_version_is_foundation() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.0.0");
}

#[cfg(feature = "cache")]
#[test]
fn cache_feature_enabled() {
    assert!(cfg!(feature = "cache"));
}

#[cfg(feature = "offline")]
#[test]
fn offline_feature_enabled() {
    assert!(cfg!(feature = "offline"));
}

#[cfg(feature = "service")]
#[test]
fn service_feature_enabled() {
    assert!(cfg!(feature = "service"));
}

#[cfg(feature = "axum")]
#[test]
fn axum_feature_enabled() {
    assert!(cfg!(feature = "axum"));
}
