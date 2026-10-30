//! What the framework promises about secrets, and what it refuses to promise.
//!
//! A secret is redacted in the places the framework owns: a diagnostic it writes, a
//! preview it draws, a test record it keeps. Provenance says where a value came from
//! and holds no value at all, so it cannot leak one.
//!
//! Every canary here is a literal that must never appear in output. Assertions about
//! leakage deliberately do not include the text in their failure message: a test that
//! prints the canary to report its own failure has already leaked it.

use std::panic;
use std::str::FromStr;

use clifacet::Command;
use clifacet::example::{self, Example, ExampleInput};
use clifacet::harness::{Outcome, Parity, Recording};
use clifacet::policy::Capabilities;
use clifacet::prompt::Scripted;
use clifacet::provenance::{Provenance, Source};
use clifacet::secret::{REDACTED, Secret};

/// A value that must not survive into any framework-owned output.
const CANARY: &str = "sk-canary-7f3a9d";

/// A credential the example refuses.
fn revoked() -> String {
    format!("revoked-{CANARY}")
}

/// Runs the example with flags, returning its exit code, data and diagnostic.
fn run(flags: &[&str]) -> (i32, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        std::iter::once(String::from("example")).chain(flags.iter().map(|flag| (*flag).to_owned())),
        Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn a_secret_never_prints_itself() {
    let secret = Secret::new(String::from(CANARY));

    assert_eq!(format!("{secret:?}"), "Secret([redacted])");
    assert_eq!(format!("{secret}"), REDACTED);
    assert!(!format!("{secret:?}{secret}").contains(CANARY));
}

#[test]
fn a_secret_inside_a_struct_is_redacted_with_it() {
    // This is the shape a test record takes: the input's own `Debug`, written by the
    // application but produced by the framework's type.
    let input = ExampleInput {
        name: String::from("ada"),
        repeat: 1,
        explode: false,
        level: example::Level::Basic,
        token: Some(Secret::new(String::from(CANARY))),
    };

    let shown = format!("{input:?}");

    assert!(
        shown.contains("Some(Secret([redacted]))"),
        "the field still reads as present"
    );
    assert!(!shown.contains(CANARY), "the value is not in the record");
}

#[test]
fn a_secret_is_reachable_only_by_asking() {
    let secret = Secret::from_str(CANARY).expect("any string is a secret");

    assert_eq!(secret.expose(), CANARY);
    assert_eq!(secret.clone().into_inner(), CANARY);
    assert_eq!(secret, Secret::new(String::from(CANARY)));
    assert_ne!(secret, Secret::new(String::from("other")));
}

#[test]
fn a_rejected_credential_is_redacted_in_the_diagnostic_the_application_writes() {
    let token = revoked();
    let (code, data, diagnostic) = run(&["--token", &token, "--name", "ada"]);

    assert_eq!(code, example::EXIT_EXECUTION_FAILED);
    assert_eq!(data, "");
    assert!(
        diagnostic.contains("was rejected"),
        "the diagnostic says what happened: it should name the credential, not print it"
    );
    assert!(
        diagnostic.contains(REDACTED),
        "the value is replaced by the framework's marker"
    );
    assert!(
        !diagnostic.contains(CANARY),
        "the canary is not in the diagnostic"
    );
}

#[test]
fn an_accepted_credential_changes_nothing_else() {
    let (code, data, diagnostic) = run(&["--token", "sk-live-ok", "--name", "ada"]);

    assert_eq!(code, example::EXIT_SUCCESS);
    assert_eq!(data, "hello, ada\n");
    assert_eq!(
        diagnostic, "",
        "a healthy run says nothing about the credential"
    );
}

#[test]
fn a_blank_credential_is_absent_rather_than_empty() {
    use clifacet::Facet;
    use clifacet::clap::Parser;

    let input = example::Cli::try_parse_from(["example", "--token", "  "])
        .expect("a blank value parses")
        .validate()
        .expect("a blank credential is not an error");

    assert!(input.token.is_none(), "whitespace is not a credential");
}

#[test]
fn provenance_records_a_source_and_never_a_value() {
    let mut provenance = Provenance::default();
    provenance.record("name", Source::Argv);
    provenance.record_secret("token", Source::Prompt);

    let origin = provenance
        .source_of("token")
        .expect("the token was recorded");

    assert_eq!(origin, Source::Prompt);
    assert_eq!(provenance.len(), 2);
    assert!(!provenance.is_empty());

    let shown = provenance.to_string();
    assert!(
        shown.contains("token <- prompt (secret)"),
        "the record names the source"
    );
    assert!(shown.contains("name <- argv"));
    assert!(
        !shown.contains(CANARY),
        "a record without values cannot leak one"
    );
}

#[test]
fn every_source_has_a_name_a_person_can_read() {
    assert_eq!(Source::Argv.name(), "argv");
    assert_eq!(Source::Environment.name(), "environment");
    assert_eq!(Source::Default.name(), "default");
    assert_eq!(Source::Prompt.name(), "prompt");
    assert_eq!(Source::Form.name(), "form");
    assert_eq!(Source::Form.to_string(), "form");
}

#[test]
fn a_failed_parity_comparison_does_not_print_a_secret() {
    // The effects are the framework's own record: `Recording` describes the input it
    // received, and a secret in that input is redacted by its `Debug`.
    let (command, effects) = Recording::new(Example, |input: &ExampleInput| format!("{input:?}"));

    let accepted = ExampleInput {
        name: String::from("ada"),
        repeat: 1,
        explode: false,
        level: example::Level::Basic,
        token: Some(Secret::new(String::from(CANARY))),
    };
    let mut other = accepted.clone();
    other.token = None;

    command
        .execute(accepted)
        .expect("a live credential is accepted");
    command.execute(other).expect("no credential is accepted");

    let recorded = effects.recorded();
    let reference = Outcome::measured(|_, _| 0).with_effects([recorded[0].clone()]);
    let compared = Outcome::measured(|_, _| 0).with_effects([recorded[1].clone()]);

    let message = panic::catch_unwind(|| {
        Parity::new("flags", reference)
            .compare("scripted prompt", compared)
            .assert_parity();
    })
    .expect_err("the inputs differ, so parity fails");

    let text = message
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            message
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .expect("a panic message is a string");

    assert!(
        text.contains("effects"),
        "the difference is reported rather than silent"
    );
    assert!(
        !text.contains(CANARY),
        "a failing test does not print the canary"
    );
}

#[cfg(feature = "tui")]
#[test]
fn a_secret_field_is_masked_in_a_form_preview() {
    use clifacet::tui::ratatui::Terminal;
    use clifacet::tui::ratatui::backend::TestBackend;
    use clifacet::tui::ratatui::widgets::Widget;
    use clifacet::tui::{FieldState, Form, FormState};

    let state = FormState::new(vec![
        FieldState::new("token", "API token", CANARY).mark_secret(true),
        FieldState::new("name", "Who to greet", "ada"),
    ]);

    let mut terminal = Terminal::new(TestBackend::new(60, 4)).expect("a test backend");
    terminal
        .draw(|frame| {
            let area = frame.area();
            Form::new(&state).render(area, frame.buffer_mut());
        })
        .expect("drawing into a test backend cannot fail");

    let buffer = terminal.backend().buffer().clone();
    let drawn: String = (0..buffer.area.height)
        .map(|line| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, line)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(drawn.contains(REDACTED), "the preview shows the marker");
    assert!(
        !drawn.contains(CANARY),
        "the preview does not show the secret"
    );
    assert!(
        drawn.contains("ada"),
        "a field that is not secret is drawn as usual"
    );
    // Masking is presentation: the submission still carries the value, and the fields
    // that are not secret are submitted as usual. Asserted by membership so a failure
    // does not print the canary.
    let arguments = state.arguments();
    assert!(
        arguments.contains(&String::from(CANARY)),
        "the submission still carries the value"
    );
    assert!(
        arguments.contains(&String::from("--name")),
        "a field that is not secret is submitted as usual"
    );
}

#[test]
fn an_entry_reports_what_it_recorded() {
    let mut provenance = Provenance::default();
    provenance.record_secret("token", Source::Argv);

    let [origin] = provenance.entries() else {
        panic!("one input was recorded");
    };

    assert_eq!(origin.name(), "token");
    assert_eq!(origin.source(), Source::Argv);
    assert!(origin.is_secret());
    assert!(provenance.is_secret("token"));
    assert!(
        !provenance.is_secret("name"),
        "an input that was not recorded is not secret either"
    );
    assert_eq!(provenance.source_of("name"), None);
}

#[test]
fn recording_the_same_input_twice_keeps_one_entry() {
    // A re-ask is the same input, not a second one: a record that grew on every
    // question would say more about the flow than about where the value came from.
    let mut provenance = Provenance::default();
    provenance.record("name", Source::Prompt);
    provenance.record_secret("name", Source::Argv);

    assert_eq!(provenance.len(), 1);
    assert_eq!(provenance.source_of("name"), Some(Source::Argv));
    assert!(provenance.is_secret("name"), "the later record wins");
    assert_eq!(provenance.to_string(), "name <- argv (secret)");
}

#[test]
fn an_empty_record_renders_as_nothing() {
    let provenance = Provenance::default();

    assert!(provenance.is_empty());
    assert_eq!(provenance.to_string(), "");
    assert_eq!(provenance.entries(), []);
}

#[cfg(feature = "tui")]
#[test]
fn a_generated_form_can_mark_one_of_its_fields_secret() {
    use clifacet::clap::CommandFactory;
    use clifacet::clap::Parser;
    use clifacet::tui::FormState;

    /// A command with a credential.
    #[derive(Debug, Parser)]
    #[command(name = "demo")]
    struct Demo {
        /// Who to greet
        #[arg(long, default_value = "world")]
        name: String,
        /// An API token
        #[arg(long)]
        token: Option<String>,
    }

    let mut form =
        FormState::generated(&Demo::command(), &["name", "token"]).expect("both arguments exist");

    form.mark_secret("token");
    form.mark_secret("nope");

    assert!(
        form.field("token").expect("token exists").is_secret(),
        "the named field is marked"
    );
    assert!(
        !form.field("name").expect("name exists").is_secret(),
        "the other field is not"
    );
    assert_eq!(
        form.arguments(),
        vec![String::from("--name"), String::from("world")],
        "marking a field shows nothing and submits nothing different"
    );
}
