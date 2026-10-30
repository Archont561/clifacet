//! The TUI's public components, driven headlessly.
//!
//! decision-4 asks for ratatui components an application can reuse, with the
//! generated form built *from* them rather than beside them. These tests draw into
//! a `TestBackend` and press keys, so nothing here needs a terminal — which is what
//! makes focus, submission and cancellation assertable at all.

#![cfg(feature = "tui")]

use clifacet::clap::{CommandFactory, Parser};
use clifacet::tui::ratatui::Terminal;
use clifacet::tui::ratatui::backend::TestBackend;
use clifacet::tui::ratatui::widgets::Widget;
use clifacet::tui::{Form, FormState, Key, Outcome};
use rstest::rstest;

/// A small command to generate a form from.
#[derive(Debug, Parser)]
#[command(name = "demo")]
struct Demo {
    /// Who to greet
    #[arg(long, default_value = "world")]
    name: String,

    /// How many times to greet
    #[arg(long, default_value_t = 1)]
    repeat: u8,

    /// A value with no default
    #[arg(long)]
    extra: Option<String>,
}

/// A form generated from the demo command.
fn form() -> FormState {
    FormState::generated(&Demo::command(), &["name", "repeat", "extra"])
        .expect("these fields exist on the command")
}

/// Renders the whole form into a buffer of the given size and returns it as text.
fn rendered(state: &FormState, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test backend");
    terminal
        .draw(|frame| {
            let area = frame.area();
            Form::new(state).render(area, frame.buffer_mut());
        })
        .expect("drawing into a test backend cannot fail");

    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|line| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, line)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_generated_form_mirrors_the_command() {
    let state = form();

    let names: Vec<&str> = state
        .fields()
        .iter()
        .map(clifacet::tui::FieldState::name)
        .collect();
    assert_eq!(names, ["name", "repeat", "extra"]);

    let name = state.field("name").expect("the name field exists");
    assert_eq!(
        name.label(),
        "Who to greet",
        "the help text becomes the label"
    );
    assert_eq!(
        name.value(),
        "world",
        "the clap default is the initial value"
    );

    let extra = state.field("extra").expect("the extra field exists");
    assert_eq!(
        extra.value(),
        "",
        "an argument without a default starts empty"
    );
    assert!(!extra.required(), "an Option is not required");
}

#[test]
fn a_generated_form_refuses_a_field_the_command_does_not_have() {
    let error = FormState::generated(&Demo::command(), &["name", "nope"])
        .expect_err("an unknown field is a mistake, not an empty form");

    assert!(
        error.message().contains("nope"),
        "the message names the field: {}",
        error.message()
    );
}

#[rstest]
#[case::letters(vec![Key::Char('a'), Key::Char('b')], "ab")]
#[case::bytes(vec![Key::Char('\t')], "world")]
#[case::backspace(vec![Key::Backspace], "worl")]
#[case::backspace_on_empty(vec![Key::Backspace; 8], "")]
fn typing_edits_only_the_focused_field(#[case] keys: Vec<Key>, #[case] expected: &str) {
    let mut state = form();
    let mut outcomes = Vec::new();

    for key in keys {
        outcomes.push(state.handle(key));
    }

    assert!(
        outcomes.iter().all(|outcome| *outcome == Outcome::Editing),
        "editing keys do not submit or cancel: {outcomes:?}"
    );
    assert_eq!(state.field("name").expect("name exists").value(), expected);
    assert_eq!(
        state.field("repeat").expect("repeat exists").value(),
        "1",
        "the other fields are untouched"
    );
}

#[test]
fn tab_and_backtab_move_the_focus_and_wrap() {
    let mut state = form();
    assert_eq!(state.focus(), 0);

    state.handle(Key::Tab);
    assert_eq!(state.focus(), 1);
    state.handle(Key::Tab);
    state.handle(Key::Tab);
    assert_eq!(state.focus(), 0, "focus wraps to the first field");

    state.handle(Key::BackTab);
    assert_eq!(state.focus(), 2, "and backwards from the first field");
}

#[test]
fn typing_goes_to_the_focused_field_only() {
    let mut state = form();
    state.handle(Key::Tab);
    state.handle(Key::Char('7'));
    state.handle(Key::Char('2'));

    assert_eq!(
        state.field("repeat").expect("repeat exists").value(),
        "72",
        "typing replaces the untouched default in the focused field"
    );
    assert_eq!(
        state.field("name").expect("name exists").value(),
        "world",
        "the field that lost the focus is untouched"
    );
}

#[test]
fn enter_submits_and_escape_cancels() {
    let mut state = form();

    assert_eq!(state.handle(Key::Enter), Outcome::Submit);
    assert_eq!(state.handle(Key::Escape), Outcome::Cancel);
}

#[test]
fn the_arguments_name_only_the_fields_that_were_filled() {
    let mut state = form();

    assert_eq!(
        state.arguments(),
        vec!["--name", "world", "--repeat", "1"],
        "an empty optional value is omitted rather than passed as empty"
    );

    state.handle(Key::Tab);
    state.handle(Key::Tab);
    state.handle(Key::Char('x'));
    assert_eq!(
        state.arguments(),
        vec!["--name", "world", "--repeat", "1", "--extra", "x"]
    );
}

#[test]
fn the_form_renders_its_labels_values_and_the_focused_field() {
    let text = rendered(&form(), 40, 8);

    assert!(text.contains("Who to greet"), "{text}");
    assert!(text.contains("world"), "{text}");
    assert!(text.contains("How many times to greet"), "{text}");

    let first_line = text.lines().next().expect("the form draws something");
    assert!(
        first_line.contains('>') || first_line.contains('*'),
        "the focused field is marked: {first_line:?}"
    );

    let mut moved = form();
    moved.handle(Key::Tab);
    let after = rendered(&moved, 40, 8);
    assert_ne!(text, after, "moving focus changes the drawing");
}

#[test]
fn a_message_is_rendered_below_the_fields() {
    let mut state = form();
    state.set_message("--repeat must be between 1 and 8, got 9");

    let text = rendered(&state, 60, 10);

    assert!(
        text.contains("must be between 1 and 8"),
        "a rejected answer is shown where it was typed: {text}"
    );
}

#[test]
fn a_field_can_be_rendered_on_its_own() {
    // The builder path an application uses for a custom layout (decision-4).
    let mut terminal = Terminal::new(TestBackend::new(30, 1)).expect("a test backend");
    let state = form();
    terminal
        .draw(|frame| {
            let area = frame.area();
            clifacet::tui::Field::new(state.field("name").expect("name exists"), true)
                .render(area, frame.buffer_mut());
        })
        .expect("drawing one field cannot fail");

    let buffer = terminal.backend().buffer().clone();
    let text: String = (0..buffer.area.width)
        .map(|column| buffer[(column, 0)].symbol())
        .collect();
    assert!(text.contains("world"), "{text}");
}

#[test]
fn a_form_generated_from_no_arguments_is_a_mistake() {
    let error = FormState::generated(&Demo::command(), &[])
        .expect_err("a form with nothing to ask is a bug in the application");

    assert!(
        error.message().contains("at least one argument"),
        "the message says what is wrong: {}",
        error.message()
    );
}

#[test]
fn an_argument_without_help_is_labelled_by_its_name() {
    /// A command whose second argument documents nothing.
    #[derive(Debug, Parser)]
    #[command(name = "terse")]
    struct Terse {
        /// A documented value.
        #[arg(long)]
        documented: Option<String>,
        #[arg(long, hide = true)]
        undocumented: Option<String>,
    }

    let state = FormState::generated(&Terse::command(), &["documented", "undocumented"])
        .expect("both arguments exist");

    assert_eq!(
        state
            .field("undocumented")
            .expect("undocumented exists")
            .label(),
        "undocumented",
        "a value with no help still has to be askable"
    );
}
