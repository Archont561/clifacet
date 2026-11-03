//! The derive's diagnostics are part of its contract.
//!
//! A companion derive that fails obscurely is worse than no derive: the point is
//! to prove an *existing* clap command, so the errors it can raise have to say
//! what to write instead.
#[test]
fn the_command_attribute_wraps_clap_and_reports_what_it_cannot_wrap() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/command_valid.rs");
    cases.compile_fail("tests/ui/command_missing_input.rs");
    cases.compile_fail("tests/ui/command_on_enum.rs");
}

#[test]
fn the_facet_derive_reports_what_it_cannot_bridge() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/facet_valid.rs");
    cases.pass("tests/ui/facet_valid_bridge.rs");
    cases.compile_fail("tests/ui/facet_missing_input.rs");
    cases.compile_fail("tests/ui/facet_unknown_key.rs");
    cases.compile_fail("tests/ui/facet_on_enum.rs");
    cases.compile_fail("tests/ui/facet_missing_field.rs");
}

#[test]
fn a_field_attribute_declares_an_argument_and_reports_what_it_cannot_declare() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/field_attribute_valid.rs");
    cases.compile_fail("tests/ui/field_attribute_unknown_key.rs");
    cases.compile_fail("tests/ui/field_attribute_repeated_key.rs");
}
