//! Unit tests for the derive's expansion and its diagnostics.
//!
//! `trybuild` proves what a *user* sees when something is wrong, but it compiles
//! each case in a separate process, so those runs cannot count toward this crate's
//! coverage and they cannot reach a branch in isolation. These tests call
//! [`expand`](crate::facet::expand) directly: one case per accept and per reject
//! rule, asserting the generated code or the exact message.

use syn::DeriveInput;

use crate::command::expand as expand_command;
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
    // The copy lives in the generated `TryFrom`, which `validate` delegates to.
    assert!(
        tokens.contains("impl :: core :: convert :: TryFrom < Cli > for Greeting"),
        "{tokens}"
    );
    assert!(tokens.contains("name : cli . name"), "{tokens}");
    assert!(tokens.contains("repeat : cli . repeat"), "{tokens}");
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

// --- `#[clifacet::command(...)]`: clap and the derive, in one attribute -------

/// Expands the attribute on the given source, as the macro would.
fn command(attribute: &str, source: &str) -> syn::Result<String> {
    expand_command(
        &attribute.parse().expect("the attribute parses"),
        source.parse().expect("the item parses"),
    )
    .map(|tokens| tokens.to_string())
}

/// Expands an attribute that must be rejected, returning the reported message.
fn rejected_command(attribute: &str, source: &str) -> String {
    command(attribute, source)
        .err()
        .map(|error| error.to_string())
        .expect("this attribute must be rejected")
}

#[test]
fn the_command_attribute_wraps_clap_and_the_derive_together() {
    let tokens = command(
        "input = Greeting",
        r"
        struct Cli {
            name: Option<String>,
        }
        ",
    )
    .expect("a named struct with an input type is supported");

    assert!(
        tokens.contains(" [derive (:: clap :: Parser , :: clifacet :: Facet)]"),
        "{tokens}"
    );
    assert!(tokens.contains(" [facet (input = Greeting)]"), "{tokens}");
    assert!(
        tokens.contains("struct Cli"),
        "the item is re-emitted: {tokens}"
    );
}

#[test]
fn the_command_attribute_keeps_the_item_untouched() {
    let source = r#"
        #[derive(Debug)]
        #[command(name = "example", about = "A command")]
        struct Cli {
            /// The name to greet.
            #[arg(long, alias = "nickname")]
            name: Option<String>,
        }
        "#;

    let tokens = command("input = Greeting", source).expect("a full command struct is supported");

    for kept in [
        "# [derive (Debug)]",
        "# [command (name = \"example\" , about = \"A command\")]",
        "# [doc = \" The name to greet.\"]",
        "# [arg (long , alias = \"nickname\")]",
    ] {
        assert!(tokens.contains(kept), "`{kept}` was dropped: {tokens}");
    }
}

#[test]
fn the_command_attribute_passes_a_bridge_through() {
    let tokens = command(
        "input = Greeting, bridge = example::build",
        "struct Cli { name: Option<String> }",
    )
    .expect("a bridge is supported");

    assert!(
        tokens.contains(" [facet (input = Greeting , bridge = example :: build)]"),
        "{tokens}"
    );
}

#[test]
fn the_command_attribute_requires_an_input_type_and_says_so_in_its_own_name() {
    let message = rejected_command("", "struct Cli { name: Option<String> }");

    assert!(
        message.contains("[clifacet::command(input = YourInputType)]"),
        "the message must name the attribute the caller wrote: {message}"
    );
}

#[test]
fn the_command_attribute_rejects_an_enum_because_the_bridge_needs_a_struct() {
    let message = rejected_command("input = Greeting", "enum Cli { A, B }");

    assert!(message.contains("struct"), "{message}");
}

#[test]
fn the_command_attribute_refuses_to_derive_claps_parser_twice() {
    let message = rejected_command(
        "input = Greeting",
        r"
        #[derive(clap::Parser)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("Parser"), "{message}");
    assert!(
        message.contains("[clifacet::command"),
        "the message should point at the attribute that already does it: {message}"
    );
}

#[test]
fn the_command_attribute_refuses_to_derive_facet_twice() {
    let message = rejected_command(
        "input = Greeting",
        r"
        #[derive(clifacet::Facet)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(message.contains("Facet"), "{message}");
}

#[test]
fn the_command_attribute_refuses_a_second_facet_attribute() {
    let message = rejected_command(
        "input = Greeting",
        r"
        #[facet(input = Greeting)]
        struct Cli { name: Option<String> }
        ",
    );

    assert!(
        message.contains("facet"),
        "two `#[facet(...)]` attributes would declare the input twice: {message}"
    );
}

#[test]
fn the_command_attribute_leaves_the_derive_available_on_its_own() {
    // The wrapper is sugar, not a replacement: `#[derive(Facet)]` still works, and the
    // wrapper emits exactly that derive rather than a second implementation of it.
    let wrapped = command("input = Greeting", "struct Cli { name: Option<String> }")
        .expect("the wrapper expands");
    let derived = expanded(
        r"
        #[facet(input = Greeting)]
        struct Cli { name: Option<String> }
        ",
    )
    .expect("the derive expands");

    assert!(wrapped.contains("Facet"), "{wrapped}");
    assert!(
        derived.contains("impl :: clifacet :: Facet for Cli"),
        "{derived}"
    );
}

// --- Field rules: `#[facet(..)]` on a field, and what `#[clifacet(..)]` forwards ---

#[test]
fn field_rules_are_applied_in_one_fixed_order_whatever_order_they_were_written_in() {
    let tokens = expanded(
        r#"
        #[facet(input = Greeting)]
        struct Cli {
            #[arg(long)]
            #[facet(non_empty, trim, fill = "world")]
            name: Option<String>,
        }
        "#,
    )
    .expect("the rules apply to text");

    let fill = tokens.find("__derive :: fill").expect("fill is generated");
    let trim = tokens.find("Text :: trim").expect("trim is generated");
    let non_empty = tokens
        .find("Text :: non_empty")
        .expect("non_empty is generated");
    assert!(
        fill < trim && trim < non_empty,
        "fill, trim, non_empty: {tokens}"
    );
    assert!(
        tokens.contains("flag :: < Cli > (\"name\")"),
        "names the flag lazily: {tokens}"
    );
}

#[test]
fn a_command_without_questions_collects_by_returning_itself() {
    let tokens =
        expanded("#[facet(input = Greeting)] struct Cli { name: Option<String> }").expect("valid");
    assert!(
        tokens.contains("impl :: clifacet :: prompt :: Collect for Cli"),
        "{tokens}"
    );
    assert!(tokens.contains("_session"), "nothing to ask: {tokens}");
    assert!(
        !tokens.contains("CommandFactory"),
        "no command is built: {tokens}"
    );
}

#[test]
fn questions_are_asked_by_the_id_clap_knows_the_field_by() {
    let tokens = expanded(
        r#"
        #[facet(input = Greeting)]
        struct Cli {
            #[arg(long, id = "who")]
            #[facet(ask = "Who?")]
            name: Option<String>,
            #[arg(long)]
            #[facet(confirm = "Sure?")]
            explode: bool,
        }
        "#,
    )
    .expect("valid");
    assert!(
        tokens.contains("ask_for (& mut self . name , & command , \"who\" , \"Who?\")"),
        "{tokens}"
    );
    assert!(
        tokens.contains("confirm_flag (self . explode , & command , \"explode\" , \"Sure?\")"),
        "{tokens}"
    );
}

#[test]
fn a_field_rule_is_checked_for_shape_and_combination() {
    let cases = [
        ("#[facet(trimm)]", "`trimm` is not a field key"),
        ("#[facet(a::b)]", "this key is not a field key"),
        ("#[facet(ask)]", "`ask` takes a value"),
        ("#[facet(trim = true)]", "`trim` takes no value"),
        ("#[facet(trim, trim)]", "`trim` is declared twice"),
        ("#[facet(ask = who)]", "a question is a string literal"),
        ("#[facet(confirm = 1)]", "a question is a string literal"),
        (r#"#[facet(ask = "a", confirm = "b")]"#, "either asked for"),
        (
            r#"#[facet(ask = "a", secret)]"#,
            "would echo the credential",
        ),
    ];
    for (rules, expected) in cases {
        let message = rejected(&format!(
            "#[facet(input = Greeting)] struct Cli {{ #[arg(long)] {rules} name: Option<String> }}"
        ));
        assert!(
            message.contains(expected),
            "{rules}: expected {expected:?} in: {message}"
        );
    }
}

#[test]
fn an_asked_field_has_to_be_a_named_option() {
    let message = rejected(
        r#"#[facet(input = Greeting)] struct Cli { #[arg(value_name = "N")] #[facet(ask = "Who?")] name: Option<String> }"#,
    );
    assert!(message.contains("needs a named option"), "{message}");
}

#[test]
fn a_rule_beside_a_declared_bridge_is_rejected_but_a_question_is_not() {
    let message = rejected(
        "#[facet(input = Greeting, bridge = build)] struct Cli { #[arg(long)] #[facet(fill = 1)] n: Option<u8> }",
    );
    assert!(
        message.contains("`fill` is a rule for the generated bridge"),
        "{message}"
    );

    let tokens = expanded(
        r#"#[facet(input = Greeting, bridge = build)] struct Cli { #[arg(long)] #[facet(ask = "N?")] n: Option<u8> }"#,
    )
    .expect("collection is independent of the bridge");
    assert!(tokens.contains("build (self)"), "{tokens}");
    assert!(
        !tokens.contains("TryFrom"),
        "the declared bridge replaces the generated one: {tokens}"
    );
}

#[test]
fn range_becomes_claps_ranged_parser_for_the_parsed_type() {
    let cases = [
        ("u8", "value_parser ! (u8)"),
        ("Option<u8>", "value_parser ! (u8)"),
        ("Vec<u16>", "value_parser ! (u16)"),
        ("Option<Vec<i64>>", "value_parser ! (i64)"),
        (
            "std::num::NonZeroU8",
            "value_parser ! (std :: num :: NonZeroU8)",
        ),
    ];
    for (ty, parser) in cases {
        let tokens = command(
            "input = Greeting",
            // Spaced: proc_macro2's string lexer reads `1..=8` as the float `1.`; rustc does
            // not, which `tests/ui/field_attribute_valid.rs` proves with the tight spelling.
            &format!("struct Cli {{ #[clifacet(long, range = 1 ..= 8)] n: {ty} }}"),
        )
        .expect("range is supported");
        assert!(tokens.contains(parser), "{ty}: {tokens}");
        assert!(tokens.contains(". range (1 ..= 8)"), "{ty}: {tokens}");
        assert!(
            !tokens.contains("range ="),
            "{ty}: no `range` key reaches clap: {tokens}"
        );
    }
}

#[test]
fn range_beside_a_value_parser_is_two_parsers_for_one_argument() {
    let message = rejected_command(
        "input = Greeting",
        "struct Cli { #[clifacet(long, range = 1 ..= 8, value_parser = parse)] n: u8 }",
    );
    assert!(message.contains("`range` already builds"), "{message}");
}

#[test]
fn grammar_keys_are_accepted_in_the_spelling_claps_derive_accepts() {
    let keys = [
        "required = true",
        "hide = true",
        "env",
        "env = \"APP_N\"",
        "default_value_t",
        "value_parser",
        "action",
        "visible_alias = \"count\"",
    ];
    for key in keys {
        let tokens = command(
            "input = Greeting",
            &format!("struct Cli {{ #[clifacet(long, {key})] n: u8 }}"),
        )
        .unwrap_or_else(|error| panic!("{key}: clap's own spelling is accepted: {error}"));
        assert!(tokens.contains("# [arg (long ,"), "{key}: {tokens}");
    }
}

#[test]
fn grammar_keys_in_a_spelling_claps_derive_rejects_are_reported_here() {
    let cases = [
        ("required", "`required` takes a value"),
        ("value_enum = true", "`value_enum` takes no value"),
        ("long_alias = \"x\"", "`long_alias` is not a"),
    ];
    for (key, expected) in cases {
        let message = rejected_command(
            "input = Greeting",
            &format!("struct Cli {{ #[clifacet(long, {key})] n: u8 }}"),
        );
        assert!(
            message.contains(expected),
            "{key}: expected {expected:?} in: {message}"
        );
    }
}

#[test]
fn interaction_and_normalization_keys_are_forwarded_to_the_derive() {
    let tokens = command(
        "input = Greeting",
        r#"struct Cli { #[clifacet(long, ask = "Who?", trim)] name: Option<String> }"#,
    )
    .expect("forwarded");
    assert!(
        tokens.contains("# [arg (long)]"),
        "grammar to clap: {tokens}"
    );
    assert!(
        tokens.contains("# [facet (ask = \"Who?\" , trim)]"),
        "rules to the derive: {tokens}"
    );
}
