//! Foundation smoke tests — prove the crate links under default features.
//!
//! Feature-flag compile coverage comes from CI running
//! `cargo test --all-features` and `cargo test --no-default-features`.

#[test]
fn crate_links() {
    assert_eq!(env!("CARGO_PKG_NAME"), "translaas");
}

#[test]
fn crate_version_is_foundation() {
    assert_eq!(env!("CARGO_PKG_VERSION"), "0.0.0");
}
