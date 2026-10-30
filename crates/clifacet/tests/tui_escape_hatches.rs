//! The two escape hatches decision-4 promises, driven headlessly.
//!
//! A custom field replaces how one answer is drawn without leaving the form, and a
//! custom screen replaces the whole drawing without leaving the command: both still
//! collect a draft, and both still submit it through `clap` and the same handler.
//! Neither needs a registry, a backend abstraction or a second command.

#![cfg(feature = "tui")]

use clifacet::example;
use clifacet::tui::ratatui::Terminal;
use clifacet::tui::ratatui::backend::TestBackend;
use clifacet::tui::ratatui::buffer::Buffer;
use clifacet::tui::ratatui::layout::Rect;
use clifacet::tui::ratatui::widgets::{Paragraph, Widget};
use clifacet::tui::{CustomField, FieldState, Form, FormState, Key, app};

/// A field that hides the answer it draws.
///
/// It is deliberately not a smart widget: the point is that a value the form holds
/// can be drawn by code the crate has never seen.
struct MaskedField;

impl CustomField for MaskedField {
    fn draw(&self, field: &FieldState, focused: bool, area: Rect, buffer: &mut Buffer) {
        let marker = if focused { "> " } else { "  " };
        let hidden = "*".repeat(field.value().chars().count());
        Paragraph::new(format!("{marker}{}: {hidden}", field.label())).render(area, buffer);
    }
}

/// The form the example asks, with its fields in a known order.
fn form() -> FormState {
    example::tui_form()
}

/// Renders a form into a buffer and returns every row as text.
fn rendered(form: Form<'_>, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test backend");
    terminal
        .draw(|frame| {
            let area = frame.area();
            form.render(area, frame.buffer_mut());
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
fn a_custom_field_draws_its_own_answer_inside_a_form() {
    // `--repeat` starts at its default of 1, so the mask has exactly one star to draw.
    let state = form();
    let custom = Form::new(&state).with_field("repeat", &MaskedField);
    let text = rendered(custom, 120, 8);

    assert!(
        text.contains(": *"),
        "the custom field drew the answer, masked: {text}"
    );
    assert!(
        !text.contains(": 1"),
        "the answer is hidden by the widget, not by the form: {text}"
    );
    assert!(
        text.contains("How many times"),
        "the fields the hatch did not replace are still drawn: {text}"
    );
}

#[test]
fn a_form_without_the_hatch_draws_the_plain_field() {
    let state = form();

    let text = rendered(Form::new(&state), 120, 8);

    assert!(
        text.contains(": 1"),
        "without the hatch the answer is drawn as it is: {text}"
    );
}

#[test]
fn a_custom_field_sees_the_focus_like_the_plain_one() {
    let mut state = form();
    let before = rendered(Form::new(&state).with_field("repeat", &MaskedField), 40, 8);

    state.handle(Key::Tab);

    let after = rendered(Form::new(&state).with_field("repeat", &MaskedField), 40, 8);

    assert_ne!(
        before, after,
        "a custom field is told whether it has the focus, so it can show it"
    );
    assert!(
        after.contains("> "),
        "the focused custom field is marked: {after}"
    );
}

#[test]
fn a_custom_screen_submits_through_the_same_validation() {
    let mut state = form();
    let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("a test backend");
    let keys = vec![Key::Char('a'), Key::Char('d'), Key::Char('a'), Key::Enter];

    let data = app::run_with(
        &mut terminal,
        &mut state,
        keys,
        |state: &FormState, area: Rect, buffer: &mut Buffer| {
            // A screen of the application's own: one line, drawn by hand.
            let line = format!("MY SCREEN {} values", state.fields().len());
            Paragraph::new(line).render(area, buffer);
        },
        example::tui_submit,
    )
    .expect("the example accepts a name");

    assert_eq!(
        data,
        vec![String::from("hello, ada")],
        "a custom screen reaches the same handler"
    );

    let buffer = terminal.backend().buffer().clone();
    let drawn: String = (0..buffer.area.width)
        .map(|column| buffer[(column, 0)].symbol())
        .collect();
    assert!(
        drawn.contains("MY SCREEN"),
        "the custom screen is what was drawn: {drawn}"
    );
}

#[test]
fn a_custom_screen_shows_the_same_refusal_as_the_form() {
    let mut state = form();
    let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("a test backend");
    // Move to `--repeat`, replace its default with an out-of-range value, submit.
    let keys = vec![Key::Tab, Key::Backspace, Key::Char('9'), Key::Enter];

    let failure = app::run_with(
        &mut terminal,
        &mut state,
        keys,
        |state: &FormState, area: Rect, buffer: &mut Buffer| {
            let message = state.message().unwrap_or("no refusal yet");
            Paragraph::new(format!("MY SCREEN: {message}")).render(area, buffer);
        },
        example::tui_submit,
    )
    .expect_err("9 is out of range, so the script ends on a cancellation");

    assert_eq!(
        failure.kind(),
        clifacet::FailureKind::Cancelled,
        "the form stayed open instead of running the handler"
    );
    assert_eq!(
        state.message(),
        Some("--repeat must be between 1 and 8, got 9"),
        "the refusal is the same one the flags produce"
    );

    let buffer = terminal.backend().buffer().clone();
    let drawn: String = (0..buffer.area.width)
        .map(|column| buffer[(column, 0)].symbol())
        .collect();
    assert!(
        drawn.contains("between 1 and 8"),
        "a custom screen can show the refusal it is given: {drawn}"
    );
}

#[test]
fn a_custom_screen_is_drawn_inside_the_screen_the_loop_restores() {
    let mut state = form();
    let mut screen = Vec::new();

    clifacet::tui::app::run_on_with(
        &mut screen,
        &mut state,
        std::iter::empty(),
        |_state: &FormState, area: Rect, buffer: &mut Buffer| {
            Paragraph::new("MY SCREEN").render(area, buffer);
        },
        example::tui_submit,
    )
    .expect_err("no keys means the form is cancelled");

    let written = String::from_utf8_lossy(&screen).into_owned();
    assert!(
        written.contains("\u{1b}[?1049h") && written.contains("\u{1b}[?1049l"),
        "the alternate screen is entered and left even for a custom screen"
    );
    assert!(
        written.contains("MY") && written.contains("SCREEN"),
        "the custom screen is what was drawn on it: {written}"
    );
}
