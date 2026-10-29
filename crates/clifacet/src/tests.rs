use super::VERSION;

#[test]
fn version_matches_package_metadata() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(super::version(), VERSION);
}

#[test]
fn reexported_macro_reports_matching_workspace_version() {
    assert_eq!(crate::version!(), VERSION);
}
