//! Executable description of the companion derive's promise.
//!
//! The steps drive the same public seam a consumer uses — `clap` parses, then
//! `Facet::validate` bridges to typed input — so the scenario fails if the
//! derive ever needs a private clap API or a second grammar to work.

use clifacet::clap::Parser as _;
use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Scripted, Session};
use clifacet::{Command, Facet};
use clifacet_example::{Example, ExampleInput, example_command};
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// State shared between the steps of one scenario.
#[derive(Default)]
struct AppWorld {
    alias: String,
    input: Option<ExampleInput>,
    help: String,
    declared: Option<declared::DeclaredInput>,
    asked: Vec<String>,
    failure: Option<String>,
    form: Vec<String>,
    /// The name the generic input of the command family carries.
    generic_name: Option<String>,
}

#[fixture]
fn world() -> AppWorld {
    AppWorld::default()
}

#[given("an application whose name flag also answers to {alias}")]
fn the_application_has_an_alias(world: &mut AppWorld, alias: String) {
    // Read the alias out of clap's own command: if the derive invented a grammar,
    // this is where the two would diverge.
    let command = <Example as Facet>::command();
    let name = command
        .get_arguments()
        .find(|argument| argument.get_id() == "name")
        .expect("the example command declares a name argument");
    assert!(
        name.get_all_aliases()
            .unwrap_or_default()
            .contains(&alias.as_str()),
        "clap does not report {alias:?} as an alias of --name"
    );
    world.alias = alias;
}

#[when("the user runs the command with --{flag} and {value}")]
fn the_user_runs_with_the_alias(world: &mut AppWorld, flag: String, value: String) {
    assert_eq!(
        flag, world.alias,
        "the scenario drives the alias it declared"
    );
    let cli = Example::try_parse_from(["example", &format!("--{flag}"), &value])
        .expect("an alias parses through clap");
    world.input = Some(cli.validate().expect("a valid alias value is valid input"));
}

#[then("the typed input carries the name {name:string}")]
fn the_input_carries_the_name(world: &mut AppWorld, name: String) {
    let input = world
        .input
        .as_ref()
        .expect("a previous step validated input");
    assert_eq!(input.name, name);
}

#[then("the command runs without a terminal")]
fn the_command_runs_without_a_terminal(world: &mut AppWorld) {
    let input = world.input.take().expect("a previous step validated input");
    let output = example_command()
        .execute(input)
        .expect("the command succeeds");
    let mut rendered = Vec::new();
    output
        .write_to(&mut rendered)
        .expect("rendering cannot fail");

    assert!(
        String::from_utf8(rendered)
            .expect("the example writes UTF-8")
            .contains("hello, ada"),
        "the command produced the expected data"
    );
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "An alias resolves to the same typed input"
)]
#[test]
fn an_alias_resolves_to_the_same_typed_input(world: AppWorld) {
    let _ = world;
}

/// A command declared with one attribute per field, as decision-13 describes it.
///
/// Its fields are read by clap's derive, not by hand, which is why the module allows the
/// dead-code lint a struct with no reader of its own would otherwise earn.
///
/// The reference application is declared the same way; this smaller command stays
/// because it pins things the application does not use — a raw `#[arg(..)]` beside the
/// declared fields, and a required `--repeat` with no default. Declaring it in an
/// application-shaped test is the point — this is a consumer of the public macro, not a
/// second implementation of it.
#[allow(
    dead_code,
    reason = "clap's derive is the reader, and the test drives the API"
)]
mod declared {
    use clifacet::clap::CommandFactory as _;

    #[clifacet::command(input = DeclaredInput)]
    #[command(name = "declare", about = "One attribute per field")]
    pub struct Declared {
        /// Who to greet. Defaults to `world`.
        #[clifacet(
            long,
            short = 'n',
            alias = "nickname",
            value_name = "NAME",
            ask = "Who should I greet?",
            fill = "world",
            trim,
            non_empty
        )]
        pub name: Option<String>,

        /// How many times to greet.
        #[clifacet(long, value_name = "COUNT")]
        pub repeat: u8,

        /// Anything the key table does not cover stays clap's, untouched.
        #[arg(long, value_parser = clap::value_parser!(u8).range(1..=8))]
        pub level: u8,
    }

    /// The typed input the derive builds field by field: the name is never missing and
    /// never blank, because the declaration says so.
    pub struct DeclaredInput {
        pub name: String,
        pub repeat: u8,
        pub level: u8,
    }

    /// The rendered help, which is clap's because the keys were forwarded into `#[arg]`.
    pub fn help() -> String {
        Declared::command().render_help().to_string()
    }
}

#[given("a command whose arguments are declared with one attribute per field")]
fn a_command_declares_its_arguments_once(world: &mut AppWorld) {
    // Placeholder: the given step establishes the declaration, the when step asks.
    // The declaration is the fixture: it either compiles into a clap command or the
    // scenario does not run.
    world.help = declared::help();
}

#[when("the user asks for help")]
fn the_user_asks_for_help(world: &mut AppWorld) {
    world.help = declared::help();
}

#[then("the help shows the short flag, the value name and the field's doc comment")]
fn the_help_is_built_from_the_declaration(world: &AppWorld) {
    assert!(
        world.help.contains("-n, --name <NAME>"),
        "the short flag and value name reached clap:\n{}",
        world.help
    );
    assert!(
        world.help.contains("Who to greet"),
        "the doc comment is the help text:\n{}",
        world.help
    );
}

#[then("the same declaration parses a value through its alias")]
fn the_declaration_parses_through_the_alias(_world: &mut AppWorld) {
    use clifacet::clap::Parser as _;

    let cli = declared::Declared::try_parse_from([
        "declare",
        "--nickname",
        "ada",
        "--repeat",
        "3",
        "--level",
        "2",
    ])
    .expect("the forwarded grammar parses");
    assert_eq!(cli.name.as_deref(), Some("ada"));
    assert_eq!(cli.repeat, 3);
    assert_eq!(cli.level, 2);
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "A field declared once carries its own grammar"
)]
#[test]
fn a_field_declared_once_carries_its_own_grammar(world: AppWorld) {
    let _ = world;
}

#[given("a command whose name is declared with a question, a default and trimming")]
fn a_command_declares_how_its_name_is_asked_for(_world: &mut AppWorld) {
    // The declaration in `mod declared` is the fixture: `ask`, `fill`, `trim` and
    // `non_empty` sit on the one field, beside its grammar, or the scenario does not compile.
}

/// Runs the declared command through the seams an application uses: clap parses, the
/// generated `collect` asks on a session, and the generated bridge validates.
fn run_declared(world: &mut AppWorld, capabilities: Capabilities, answers: Vec<Answer>) {
    use clifacet::clap::Parser as _;
    use clifacet::prompt::Collect as _;

    let script = Scripted::new(answers);
    let probe = script.clone();
    let plan = Policy::up_to(Mode::Interactive)
        .resolve(Request::none(), capabilities)
        .expect("this plan resolves");
    let mut session = Session::start(plan, move || Box::new(probe));
    let cli = declared::Declared::try_parse_from(["declare", "--repeat", "1", "--level", "1"])
        .expect("the declared grammar parses");

    world.declared = Some(
        cli.collect(&mut session)
            .and_then(Facet::validate)
            .expect("the declared command collects and validates"),
    );
    world.asked = script.asked();
}

#[when("the user runs it at a terminal without --name and answers {answer:string}")]
fn the_user_answers_at_a_terminal(world: &mut AppWorld, answer: String) {
    run_declared(world, Capabilities::terminal(), vec![Answer::Text(answer)]);
}

#[when("the user runs it without a terminal and without --name")]
fn the_user_runs_without_a_terminal(world: &mut AppWorld) {
    run_declared(world, Capabilities::headless(), vec![]);
}

#[then("the declared input carries the name {name:string}")]
fn the_declared_input_carries_the_name(world: &mut AppWorld, name: String) {
    let input = world
        .declared
        .as_ref()
        .expect("a previous step validated input");
    assert_eq!(input.name, name);
}

#[then("the user was asked {question:string} once")]
fn the_user_was_asked_once(world: &mut AppWorld, question: String) {
    assert_eq!(world.asked, [question]);
}

#[then("the user was asked nothing")]
fn the_user_was_asked_nothing(world: &mut AppWorld) {
    assert!(world.asked.is_empty(), "asked: {:?}", world.asked);
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "A field declared once is asked for and normalised by the same declaration"
)]
#[test]
fn a_field_declared_once_is_asked_for_and_normalised(world: AppWorld) {
    let _ = world;
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "Without a terminal the declared default fills in and nothing is asked"
)]
#[test]
fn without_a_terminal_the_declared_default_fills_in(world: AppWorld) {
    let _ = world;
}

#[given("the reference application, declared one attribute per field")]
fn the_reference_application(_world: &mut AppWorld) {
    // The fixture is `clifacet_example::Example` itself: its fields carry `#[clifacet(..)]`,
    // so the rules and the form below come from its declarations or not at all.
}

/// Validates an invocation of the reference application, keeping the failure message.
fn validation_message(argv: &[&str]) -> String {
    let cli = Example::try_parse_from(std::iter::once("example").chain(argv.iter().copied()))
        .expect("the reference grammar parses");
    let failure = cli.validate().expect_err("this invocation is refused");
    assert_eq!(failure.kind(), clifacet::FailureKind::Validation);
    failure.message().to_owned()
}

#[when("the user runs it with a blank name and too many repeats")]
fn the_user_runs_with_two_mistakes(world: &mut AppWorld) {
    world.failure = Some(validation_message(&["--name", "   ", "--repeat", "9"]));
}

#[then("validation fails with {message:string}")]
fn validation_fails_with(world: &mut AppWorld, message: String) {
    assert_eq!(world.failure.as_deref(), Some(message.as_str()));
}

#[then("a name that is only padded still meets the repeat check {message:string}")]
fn a_padded_name_reaches_the_check(_world: &mut AppWorld, message: String) {
    assert_eq!(
        validation_message(&["--name", "  ada  ", "--repeat", "9"]),
        message
    );
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "The reference application checks its input after each field's own rule"
)]
#[test]
fn the_reference_application_checks_after_the_field_rules(world: AppWorld) {
    let _ = world;
}

#[when("its form is built")]
fn its_form_is_built(world: &mut AppWorld) {
    world.form = clifacet_example::tui_form()
        .fields()
        .iter()
        .map(|field| field.name().to_owned())
        .collect();
}

#[then("the form asks for {first:string}, {second:string} and {third:string}, in that order")]
fn the_form_asks_for(world: &mut AppWorld, first: String, second: String, third: String) {
    assert_eq!(world.form, [first, second, third]);
    assert_eq!(
        world.form,
        <Example as Facet>::form_fields(),
        "the form is the declarations' list, not a second one"
    );
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "The reference application's form is generated from its declarations"
)]
#[test]
fn the_reference_form_is_generated_from_its_declarations(world: AppWorld) {
    let _ = world;
}

/// A command family that names each command's input through one trait, so the input
/// type is written as a projection — `<Greet as Family>::Input` — rather than a struct
/// path, and the struct it names is generic.
mod family {
    /// The trait the commands share: each names its own input.
    pub trait Family {
        type Input;
    }

    /// The `greet` member of the family.
    pub struct Greet;

    impl Family for Greet {
        type Input = Labelled<String>;
    }

    /// An input generic over the value it carries.
    pub struct Labelled<T> {
        pub name: T,
    }

    /// The command, naming its input through the family's trait.
    #[clifacet::command(input = <Greet as Family>::Input)]
    #[command(name = "greet")]
    pub struct GreetCommand {
        /// Who to greet. Defaults to `world`.
        #[clifacet(long, fill = "world")]
        pub name: Option<String>,
    }
}

#[given("a command whose input is written {written:string}, a generic type")]
fn a_command_whose_input_is_a_projection(written: String) {
    // The type `validate` returns is the one the projection names.
    let built = std::any::type_name::<<family::GreetCommand as Facet>::Input>();
    assert!(
        built.ends_with("family::Labelled<alloc::string::String>"),
        "{written} should name the generic input, but `validate` returns {built}"
    );
}

#[when("the user runs that command with --name {name:string}")]
fn the_user_runs_that_command(world: &mut AppWorld, name: String) {
    let cli = family::GreetCommand::try_parse_from(["greet", "--name", &name])
        .expect("the flag parses through clap");
    world.generic_name = Some(cli.validate().expect("a name is valid input").name);
}

#[then("the generic input carries the name {name:string}")]
fn the_generic_input_carries_the_name(world: &mut AppWorld, name: String) {
    assert_eq!(world.generic_name, Some(name));
}

#[then("without --name the declared default fills it in as {name:string}")]
fn the_declared_default_fills_the_generic_input(name: String) {
    let input = family::GreetCommand::try_parse_from(["greet"])
        .expect("--name is optional")
        .validate()
        .expect("the default is valid input");
    assert_eq!(input.name, name);
}

#[scenario(
    path = "features/facet_bridge.feature",
    name = "An input named through a trait the commands share is built from the declarations"
)]
#[test]
fn an_input_named_through_a_trait_is_built_from_the_declarations(world: AppWorld) {
    let _ = world;
}
