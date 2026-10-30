//! The public `ratatui` components, and the generated form built from them.
//!
//! decision-4 is explicit: the generated form must be assembled from the same
//! public widgets an application can use. [`Form`] therefore renders [`Field`]s and
//! nothing else, so anything that renders correctly inside a generated form also
//! renders correctly in a hand-written screen — there is no private drawing code
//! that could drift away from the public one.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use super::state::{FieldState, FormState};

/// The marker shown in front of the field with the focus.
const FOCUS_MARKER: &str = "> ";

/// The marker shown in front of every other field.
const IDLE_MARKER: &str = "  ";

/// One labelled value, with an optional cursor.
///
/// It is public because an application that wants a different layout still wants
/// the same field: label, value, and a marker showing where typing goes.
#[derive(Debug, Clone, Copy)]
pub struct Field<'a> {
    field: &'a FieldState,
    focused: bool,
}

impl<'a> Field<'a> {
    /// Draws this field, marking it as focused or not.
    #[must_use]
    pub const fn new(field: &'a FieldState, focused: bool) -> Self {
        Self { field, focused }
    }

    /// The line this field renders to.
    fn line(&self) -> Line<'a> {
        let marker = if self.focused {
            FOCUS_MARKER
        } else {
            IDLE_MARKER
        };
        let value_style = if self.field.is_empty() {
            // An empty field is a prompt, not an empty answer: show what is missing.
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default()
        };
        let shown = if self.field.is_empty() {
            String::from("…")
        } else {
            self.field.value().to_owned()
        };

        Line::from(vec![
            Span::styled(marker, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(self.field.label()),
            Span::raw(": "),
            Span::styled(shown, value_style),
        ])
    }
}

impl Widget for Field<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        Paragraph::new(self.line()).render(area, buffer);
    }
}

/// Every field of a form, with the last refusal underneath.
///
/// The hint line is part of the component because it is part of the promise: a
/// rejected answer is shown where it was typed, not printed and forgotten.
#[derive(Debug, Clone, Copy)]
pub struct Form<'a> {
    state: &'a FormState,
}

impl<'a> Form<'a> {
    /// Draws this form.
    #[must_use]
    pub const fn new(state: &'a FormState) -> Self {
        Self { state }
    }

    /// The lines the form renders, top to bottom.
    fn lines(self) -> Vec<Line<'a>> {
        let mut lines: Vec<Line<'a>> = self
            .state
            .fields()
            .iter()
            .enumerate()
            .map(|(index, field)| Field::new(field, index == self.state.focus()).line())
            .collect();

        if let Some(message) = self.state.message() {
            lines.push(Line::from(Span::styled(
                message.to_owned(),
                Style::default().add_modifier(Modifier::BOLD),
            )));
        }
        lines.push(Line::from(Span::styled(
            "Tab: next field   Enter: submit   Esc: cancel",
            Style::default().add_modifier(Modifier::DIM),
        )));
        lines
    }
}

impl Widget for Form<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        Paragraph::new(self.lines()).render(area, buffer);
    }
}
