//! A screen of the application's own, built from the crate's own escape hatches.
//!
//! Two things are replaced and nothing else is: the name field is drawn by a widget
//! this file owns, and the whole screen is drawn by [`screen`] instead of by
//! `tui::Form`. Editing, focus, refusal and submission are the shared loop, so what
//! is typed still goes through `clap` and the example's handler.
//!
//! ```sh
//! pixi run --frozen -x cargo run --features tui --example tui_custom_screen
//! ```

use clifacet::example;
use clifacet::tui::ratatui::buffer::Buffer;
use clifacet::tui::ratatui::layout::Rect;
use clifacet::tui::ratatui::style::{Modifier, Style};
use clifacet::tui::ratatui::widgets::{Paragraph, Widget};
use clifacet::tui::{CustomField, FieldState, Form, FormState, app};

/// Draws an answer as stars, so a screen recording does not leak it.
struct Masked;

impl CustomField for Masked {
    fn draw(&self, field: &FieldState, focused: bool, area: Rect, buffer: &mut Buffer) {
        let marker = if focused { "> " } else { "  " };
        let hidden = "*".repeat(field.value().chars().count());
        Paragraph::new(format!("{marker}{}: {hidden}", field.label())).render(area, buffer);
    }
}

/// The application's own screen: a title, then the form, then the last refusal.
fn screen(state: &FormState, area: Rect, buffer: &mut Buffer) {
    let title = Rect { height: 1, ..area };
    Paragraph::new("clifacet — a screen of your own")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .render(title, buffer);

    let body = Rect {
        y: area.y.saturating_add(2),
        height: area.height.saturating_sub(2),
        ..area
    };
    Form::new(state)
        .with_field("name", &Masked)
        .render(body, buffer);
}

fn main() {
    let mut state = example::tui_form();
    let outcome = app::run_on_with(
        std::io::stdout(),
        &mut state,
        app::live_keys(),
        screen,
        example::tui_submit,
    );

    match outcome {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(failure) => {
            eprintln!("example: {failure}");
            std::process::exit(example::exit_code(&failure));
        }
    }
}
