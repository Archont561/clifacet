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

#[test]
fn a_field_rule_on_the_wrong_field_fails_to_compile_and_says_why() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/field_confirm_not_bool.rs");
    cases.compile_fail("tests/ui/field_ask_not_option.rs");
    cases.compile_fail("tests/ui/field_ask_secret.rs");
    cases.compile_fail("tests/ui/field_ask_positional.rs");
    cases.compile_fail("tests/ui/field_rule_wrong_type.rs");
    cases.compile_fail("tests/ui/field_rule_beside_bridge.rs");
    cases.compile_fail("tests/ui/field_form_positional.rs");
    cases.compile_fail("tests/ui/field_skip_with_rule.rs");
    cases.compile_fail("tests/ui/facet_check_beside_bridge.rs");
}

#[test]
fn an_input_type_the_generated_impls_cannot_name_is_rejected_at_its_tokens() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/input_names_self.rs");
    cases.compile_fail("tests/ui/input_undeclared_lifetime.rs");
    cases.compile_fail("tests/ui/input_elided_lifetime.rs");
    cases.compile_fail("tests/ui/input_inferred.rs");
    cases.compile_fail("tests/ui/input_impl_trait.rs");
    cases.compile_fail("tests/ui/input_not_a_struct.rs");
    cases.compile_fail("tests/ui/input_not_a_type.rs");
}
