//! Compile-time tests for autohands-macros using trybuild.

#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/extension_basic.rs");
    t.pass("tests/ui/extension_minimal.rs");
    t.pass("tests/ui/extension_with_default_version.rs");
    t.pass("tests/ui/extension_with_fields.rs");
}
