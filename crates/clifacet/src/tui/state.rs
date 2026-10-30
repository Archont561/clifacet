//! What a full-screen form holds, and what a keystroke does to it.
//!
//! The state machine is deliberately free of `ratatui`: a [`Key`] is a key, not a
//! crossterm event, and editing a value never needs a screen. That is what lets the
//! form be driven headlessly, and it keeps the drawing in [`crate::tui::widgets`]
//! where a look change cannot alter behaviour.

use clap::Command;

use crate::failure::Failure;

/// One keystroke the form understands.
///
/// A frontend maps its own events onto these, so the state machine does not depend
/// on a terminal library — and a test can press keys without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A printable character.
    Char(char),
    /// Delete the last character of the focused value.
    Backspace,
    /// Focus the next field, wrapping at the end.
    Tab,
    /// Focus the previous field, wrapping at the start.
    BackTab,
    /// Submit the form.
    Enter,
    /// Leave the form without submitting.
    Escape,
}

/// What the form decided about a keystroke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Stay in the form; the keystroke only changed it.
    Editing,
    /// The user asked to submit. Validation decides what happens next.
    Submit,
    /// The user left the form.
    Cancel,
}

/// One question in the form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldState {
    name: String,
    label: String,
    value: String,
    required: bool,
    /// Whether the value must not be drawn. Presentation only: the submission still
    /// carries it, because masking is about what a screen shows, not what a command
    /// receives.
    secret: bool,
    /// Whether `value` is still the one the application chose rather than something
    /// the user typed. Typing replaces a default; backspace edits it like any other
    /// text, because deleting a character from a default is an edit, not an answer.
    untouched: bool,
}

impl FieldState {
    /// Builds a field directly, for a layout the command metadata cannot describe.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        label: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        let value = value.into();
        Self {
            name: name.into(),
            label: label.into(),
            untouched: !value.is_empty(),
            value,
            required: false,
            secret: false,
        }
    }

    /// Marks this field as required, which the form uses when it builds arguments.
    #[must_use]
    pub fn mark_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Marks this field as secret, so a preview draws [`crate::secret::REDACTED`]
    /// instead of the value.
    #[must_use]
    pub const fn mark_secret(mut self, secret: bool) -> Self {
        self.secret = secret;
        self
    }

    /// Whether the value must not be drawn.
    #[must_use]
    pub const fn is_secret(&self) -> bool {
        self.secret
    }

    /// The `clap` argument id this field answers for.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The question shown to the user.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// The value being edited.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Whether the command requires this argument.
    #[must_use]
    pub const fn required(&self) -> bool {
        self.required
    }

    /// Whether the value is still empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Adds a character, replacing a default the user has not touched.
    ///
    /// A prefilled value is an offer, not an answer: appending to it would turn
    /// "change 1 to 3" into "delete the 1 first", which is the kind of small tax that
    /// makes a form worse than the flags it replaces.
    fn push(&mut self, character: char) {
        if self.untouched {
            self.value.clear();
            self.untouched = false;
        }
        self.value.push(character);
    }

    fn pop(&mut self) {
        self.untouched = false;
        self.value.pop();
    }
}

/// The form: its fields, where the cursor is, and the last refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormState {
    fields: Vec<FieldState>,
    focus: usize,
    message: Option<String>,
}

impl FormState {
    /// Builds a form from fields, for an application that does its own layout.
    ///
    /// # Panics
    ///
    /// Panics when `fields` is empty: a form with nothing to ask cannot be submitted
    /// and would look like a bug in the terminal.
    #[must_use]
    pub fn new(fields: Vec<FieldState>) -> Self {
        assert!(!fields.is_empty(), "a form needs at least one field");
        Self {
            fields,
            focus: 0,
            message: None,
        }
    }

    /// Generates a form from a clap command, one field per named argument.
    ///
    /// The list of arguments is explicit on purpose: which values a screen asks for
    /// is a design decision, and guessing it from the grammar would put the
    /// interaction flags of a command in front of a user who already chose a mode.
    /// The label comes from the argument's help, and the initial value from its
    /// default.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::execution`] when a name is not an argument of `command`:
    /// that is a mistake in the application, not something a user can fix.
    pub fn generated(command: &Command, names: &[&str]) -> Result<Self, Failure> {
        let mut fields = Vec::with_capacity(names.len());
        for name in names {
            let argument = command
                .get_arguments()
                .find(|argument| argument.get_id() == *name)
                .ok_or_else(|| {
                    Failure::execution(format!(
                        "the command {command_name} has no argument named {name:?}",
                        command_name = command.get_name()
                    ))
                })?;

            let label = argument
                .get_help()
                .map_or_else(|| (*name).to_owned(), ToString::to_string);
            let value = argument
                .get_default_values()
                .first()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_default();

            fields.push(
                FieldState::new(*name, label, value).mark_required(argument.is_required_set()),
            );
        }

        if fields.is_empty() {
            return Err(Failure::execution(
                "a generated form needs at least one argument to ask for",
            ));
        }
        Ok(Self::new(fields))
    }

    /// The fields, in the order they are shown.
    #[must_use]
    pub fn fields(&self) -> &[FieldState] {
        &self.fields
    }

    /// Marks the named field as secret, if the form has it.
    ///
    /// A name the form does not ask for is ignored: the field list stays the single
    /// source of truth for what exists.
    pub fn mark_secret(&mut self, name: &str) -> &mut Self {
        if let Some(field) = self.fields.iter_mut().find(|field| field.name == name) {
            field.secret = true;
        }
        self
    }

    /// The field answering for this argument, if the form has one.
    #[must_use]
    pub fn field(&self, name: &str) -> Option<&FieldState> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// Which field has the focus.
    #[must_use]
    pub const fn focus(&self) -> usize {
        self.focus
    }

    /// The last refusal, shown under the fields.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Shows a refusal on the form.
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    /// Applies one keystroke.
    pub fn handle(&mut self, key: Key) -> Outcome {
        match key {
            Key::Char(character) => {
                // Every editable key has its own variant, so a control character here
                // is a terminal artefact rather than an answer — typing it would only
                // put something unrenderable in a value.
                if !character.is_control()
                    && let Some(field) = self.fields.get_mut(self.focus)
                {
                    field.push(character);
                }
                // A refusal belongs to the answer that caused it, so editing clears it.
                self.message = None;
                Outcome::Editing
            }
            Key::Backspace => {
                if let Some(field) = self.fields.get_mut(self.focus) {
                    field.pop();
                }
                self.message = None;
                Outcome::Editing
            }
            Key::Tab => {
                self.focus = (self.focus + 1) % self.fields.len();
                Outcome::Editing
            }
            Key::BackTab => {
                self.focus = (self.focus + self.fields.len() - 1) % self.fields.len();
                Outcome::Editing
            }
            Key::Enter => Outcome::Submit,
            Key::Escape => Outcome::Cancel,
        }
    }

    /// The arguments these answers describe, ready for the command's own parser.
    ///
    /// An empty optional value is omitted rather than passed as an empty string, so
    /// a field left untouched behaves exactly as if the flag had not been typed.
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        let mut arguments = Vec::new();
        for field in &self.fields {
            if field.is_empty() && !field.required {
                continue;
            }
            arguments.push(format!("--{}", field.name));
            arguments.push(field.value.clone());
        }
        arguments
    }
}
