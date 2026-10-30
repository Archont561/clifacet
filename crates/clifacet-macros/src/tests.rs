//! Unit tests for the derive's expansion and its diagnostics.
//!
//! `trybuild` proves what a *user* sees when something is wrong, but it compiles
//! each case in a separate process, so those runs cannot count toward this crate's
//! coverage and they cannot reach a branch in isolation. These tests call
//! [`expand`](crate::facet::expand) directly: one case per accept and per reject
//! rule, asserting the generated code or the exact message.

use syn::DeriveInput;

use crate::facet::expand;

/// Expands a derive on the given source, as the macro would.
fn expanded(source: &str) -> syn::Result<String> {
    let input: DeriveInput = syn::parse_str(source).expect("the test source parses");
    expand(&input).map(|tokens| tokens.to_string())
}

/// Expands a derive that must fail, returning the reported message.
fn rejected(source: &str) -> String {
    expanded(source)
        .err()
        .map(|error| error.to_string())
        .expect("this derive must be rejected")
}

#[test]
fn a_declared_input_is_built_field_by_field() {
    let tokens = expanded(
        r"
        #[facet(input = Greeting)]
        struct Cli {
            name: Option<String>,
            repeat: u8,
        }
        ",
    )
    .expect("a named struct with an input type is supported");

    assert!(
        tokens.contains("impl :: clifacet :: Facet for Cli"),
        "{tokens}"
    );
    assert!(tokens.contains("type Input = Greeting"), "{tokens}");
    assert!(tokens.contains("name : self . name"), "{tokens}");
    assert!(tokens.contains("repeat : self . repeat"), "{tokens}");
    assert!(
        !tokens.contains("Greeting ::"),
        "no conversion is invented: {tokens}"
    );
}

#[test]
fn a_declared_bridge_replaces_the_field_mirror() {
    let tokens = expanded(
        r"
        #[facet(input = Greeting, bridge = example::build)]
        struct Cli {
            name: Option<String>,
        }
        ",
    )
    .expect("a bridge is supported");

    assert!(tokens.contains("example :: build (self)"), "{tokens}");
    assert!(
        !tokens.contains("self . name"),
        "the bridge owns the mapping: {tokens}"
    );
}

#[test]
fn a_struct_without_fields_still_bridges() {
    let tokens = expanded(
        r"
        #[facet(input = Nothing)]
        struct Cli {}
        ",
    )
    .expect("an empty struct has nothing to copy");

    assert!(tokens.contains("Nothing { }"), "{tokens}");
}

#[test]
fn other_attributes_are_ignored() {
    let tokens = expanded(
        r"
        #[derive(Clone)]
        #[facet(input = Greeting)]
        struct Cli { name: Option<String> }
        ",
    )
    .expect("attributes this derive does not own are clap's business");

    assert!(tokens.contains("type Input = Greeting"), "{tokens}");
}

#[test]
fn a_missing_input_type_is_named_in_the_error() {
    let message = rejected("struct Cli { name: Option<String> }");

    assert!(
        message.contains("#[facet(input = YourInputType)]"),
        "the message must say what to write: {message}"
    );
}

#[test]
fn an_empty_attribute_is_a_missing_input_type() {
    let message = rejected(
        r"
        #[facet()]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(
        message.contains("#[facet(input = YourInputType)]"),
        "{message}"
    );
}

#[test]
fn an_unknown_key_lists_the_supported_ones() {
    let message = rejected(
        r"
        #[facet(input = Greeting, magic = example::item)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("unsupported"), "{message}");
    assert!(
        message.contains("`magic`"),
        "the offending key is named: {message}"
    );
    assert!(
        message.contains("`input`, `bridge`"),
        "the alternatives are listed: {message}"
    );
}

#[test]
fn a_key_that_is_not_an_identifier_is_rejected() {
    let message = rejected(
        r"
        #[facet(input = Greeting, some::path = example::item)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("supported keys"), "{message}");
}

#[test]
fn a_bare_entry_is_rejected() {
    let message = rejected(
        r"
        #[facet(input)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("`key = value` pairs"), "{message}");
}

#[test]
fn a_value_that_is_not_a_path_is_rejected() {
    let message = rejected(
        r"
        #[facet(input = Greeting, bridge = true)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("takes a path"), "{message}");
    assert!(
        message.contains("bridge"),
        "the offending key is named: {message}"
    );
}

#[test]
fn a_duplicated_key_is_rejected() {
    let message = rejected(
        r"
        #[facet(input = Greeting, input = Other)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("declared more than once"), "{message}");
}

#[test]
fn an_enum_is_rejected_as_a_command() {
    let message = rejected(
        r"
        #[facet(input = Greeting)]
        enum Cli { Run }
        ",
    );

    assert!(message.contains("enums and unions"), "{message}");
}

#[test]
fn a_union_is_rejected_as_a_command() {
    let message = rejected(
        r"
        #[facet(input = Greeting)]
        union Cli { value: u8 }
        ",
    );

    assert!(message.contains("enums and unions"), "{message}");
}

#[test]
fn a_tuple_struct_is_rejected_as_a_command() {
    let message = rejected(
        r"
        #[facet(input = Greeting)]
        struct Cli(Option<String>);
        ",
    );

    assert!(message.contains("tuple and unit structs"), "{message}");
}

#[test]
fn a_unit_struct_is_rejected_as_a_command() {
    let message = rejected(
        r"
        #[facet(input = Greeting)]
        struct Cli;
        ",
    );

    assert!(message.contains("tuple and unit structs"), "{message}");
}
