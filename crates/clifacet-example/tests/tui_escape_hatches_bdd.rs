//! The escape hatches' user-visible promise, driven without a terminal.
//!
//! `app::run_with` takes the drawing as a parameter and `Form::with_field` takes the
//! widget, so both hatches are scenarios rather than screenshots: they are drawn into
//! a `TestBackend` and submitted through the example's own handler.

use clifacet::prompt::Scripted;
use clifacet::tui::ratatui::Terminal;
use clifacet::tui::ratatui::backend::TestBackend;
use clifacet::tui::ratatui::buffer::Buffer;
use clifacet::tui::ratatui::layout::Rect;
use clifacet::tui::ratatui::widgets::{Paragraph, Widget};
use clifacet::tui::{CustomField, FieldState, Form, FormState, Key, app};
use clifacet_example as example;
use rstest::fixture;
use rstest_bdd_macros::{given, scenario, then, when};

/// A field that hides what it draws.
struct Masked;

impl CustomField for Masked {
    fn draw(&self, field: &FieldState, focused: bool, area: Rect, buffer: &mut Buffer) {
        let marker = if focused { "> " } else { "  " };
        let hidden = "*".repeat(field.value().chars().count());
        Paragraph::new(format!("{marker}{}: {hidden}", field.label())).render(area, buffer);
    }
}

/// What one scenario has seen so far.
struct World {
    state: FormState,
    masked_field: Option<String>,
    screen: String,
    keys: Vec<Key>,
    code: i32,
    data: String,
}

impl Default for World {
    fn default() -> Self {
        Self {
            state: example::tui_form(),
            masked_field: None,
            screen: String::new(),
            keys: Vec::new(),
            code: example::EXIT_SUCCESS,
            data: String::new(),
        }
    }
}

#[fixture]
fn world() -> World {
    World::default()
}

#[given("a form with a masked field for {field}")]
fn a_form_with_a_masked_field(world: &mut World, field: String) {
    assert!(
        world.state.field(&field).is_some(),
        "the example's form asks for {field}"
    );
    world.masked_field = Some(field);
}

#[when("the form is drawn")]
fn the_form_is_drawn(world: &mut World) {
    let mut form = Form::new(&world.state);
    if let Some(name) = world.masked_field.as_deref() {
        form = form.with_field(name, &Masked);
    }

    let mut terminal = Terminal::new(TestBackend::new(120, 8)).expect("a test backend");
    terminal
        .draw(|frame| {
            let area = frame.area();
            form.render(area, frame.buffer_mut());
        })
        .expect("drawing into a test backend cannot fail");

    let buffer = terminal.backend().buffer().clone();
    world.screen = (0..buffer.area.height)
        .map(|line| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, line)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
}

#[then("the masked answer is shown")]
fn the_masked_answer_is_shown(world: &World) {
    assert!(
        world.screen.contains(": *"),
        "the custom field hid the answer: {}",
        world.screen
    );
    assert!(
        !world.screen.contains(": 1"),
        "the default is not drawn twice: {}",
        world.screen
    );
}

#[then("the other fields are drawn as usual")]
fn the_other_fields_are_drawn(world: &World) {
    assert!(
        world.screen.contains("How much detail"),
        "the fields the hatch did not replace are unchanged: {}",
        world.screen
    );
}

#[given("a screen of my own")]
fn a_screen_of_my_own(world: &mut World) {
    // Nothing to prepare: the screen is the closure the next step runs.
    assert!(!world.state.fields().is_empty());
}

#[when("the user types {text} and submits")]
fn the_user_types_and_submits(world: &mut World, text: String) {
    world.keys = text
        .chars()
        .map(Key::Char)
        .chain(std::iter::once(Key::Enter))
        .collect();

    let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("a test backend");
    let mut out = Vec::new();

    let outcome = app::run_with(
        &mut terminal,
        &mut world.state,
        world.keys.iter().copied(),
        |state: &FormState, area: Rect, buffer: &mut Buffer| {
            let line = format!("MY SCREEN {} values", state.fields().len());
            Paragraph::new(line).render(area, buffer);
        },
        example::tui_submit,
    );

    match outcome {
        Ok(lines) => {
            world.code = example::EXIT_SUCCESS;
            for line in lines {
                out.extend_from_slice(line.as_bytes());
                out.push(b'\n');
            }
        }
        Err(failure) => {
            // A cancelled scenario is a failure of the script, not of the step.
            panic!("the screen did not submit: {failure}");
        }
    }
    world.data = String::from_utf8_lossy(&out).into_owned();
}

#[then("the data is what --name {name} would produce")]
fn the_data_matches_the_flag(world: &World, name: String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = example::main_with(
        [String::from("example"), String::from("--name"), name],
        clifacet::policy::Capabilities::headless(),
        || Box::new(Scripted::default()),
        &mut out,
        &mut err,
    );

    assert_eq!(
        world.data,
        String::from_utf8_lossy(&out),
        "a custom screen reaches the same handler"
    );
    assert_eq!(world.code, code);
}

#[then("the exit code says the run succeeded")]
fn the_run_succeeded(world: &World) {
    assert_eq!(world.code, example::EXIT_SUCCESS);
}

#[scenario(
    path = "tests/features/tui_escape_hatches.feature",
    name = "A custom field draws inside the generated form"
)]
#[test]
fn a_custom_field_draws_inside_the_form(world: World) {
    let _ = world;
}

#[scenario(
    path = "tests/features/tui_escape_hatches.feature",
    name = "A custom screen submits to the same command"
)]
#[test]
fn a_custom_screen_submits(world: World) {
    let _ = world;
}
