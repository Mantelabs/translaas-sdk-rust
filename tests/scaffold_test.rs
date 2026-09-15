//! Foundation smoke tests — prove the crate links under default features.
//!
//! Feature-flag compile coverage comes from CI running
//! `cargo test --all-features` and `cargo test --no-default-features`.

#[test]
fn crate_links() {
    assert_eq!(env!("CARGO_PKG_NAME"), "translaas");
}

#[test]
fn crate_version_matches_manifest() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.4.0-beta");
}

#[cfg(feature = "service")]
#[test]
fn default_features_include_service() {
    let _ = translaas::Service::<translaas::client::Client>::new;
}

#[cfg(feature = "blocking")]
#[test]
fn blocking_feature_exports_builder() {
    let _ = translaas::blocking::ClientBuilder::new;
}
