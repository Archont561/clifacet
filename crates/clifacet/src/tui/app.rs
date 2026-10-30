//! The loop that draws the form, reads keys and submits.
//!
//! It takes a terminal and an iterator of [`Key`]s, so the same code runs a real
//! session and a scripted one. The screen it draws on is `ratatui`'s; the keys are
//! this crate's; the submission is the caller's — which is what keeps the TUI from
//! owning validation or the handler.

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;
use ratatui::widgets::Widget;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::state::{FormState, Key, Outcome};
use super::widgets::Form;
use crate::failure::Failure;

/// Where an off-screen writer is drawn: no terminal, no size, so a conventional one.
const OFF_SCREEN: Rect = Rect::new(0, 0, 80, 24);

/// Draws the form until it is submitted or abandoned.
///
/// `submit` receives the form and answers with the data the run produced, or with
/// the refusal to show on the form. Returning `Err` keeps the form open, so a
/// rejected answer can be corrected instead of ending the run — that is the whole
/// difference between a form and a command line.
///
/// # Errors
///
/// Returns [`Failure::cancelled`] when the user leaves the form or the keys run out,
/// and [`Failure::execution`] when the terminal itself fails.
pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &mut FormState,
    keys: impl IntoIterator<Item = Key>,
    mut submit: impl FnMut(&FormState) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, Failure> {
    let mut keys = keys.into_iter();

    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();
                Form::new(state).render(area, frame.buffer_mut());
            })
            .map_err(|error| Failure::execution(format!("cannot draw the form: {error}")))?;

        let Some(key) = keys.next() else {
            // The input ended with nobody submitting: that is a cancellation, not a
            // submission of whatever happened to be typed.
            return Err(Failure::cancelled(
                "the form was cancelled: the input ended before it was submitted",
            ));
        };

        match state.handle(key) {
            Outcome::Editing => (),
            Outcome::Cancel => {
                return Err(Failure::cancelled(
                    "the form was cancelled: the user left it without submitting",
                ));
            }
            Outcome::Submit => match submit(state) {
                Ok(data) => return Ok(data),
                // Shown on the form, and the loop keeps going.
                Err(message) => state.set_message(message),
            },
        }
    }
}

/// Runs the form on a writer, inside the terminal's alternate screen.
///
/// The screen is entered and left here, and left on the way out of *every* path —
/// a cancellation must not leave a user's terminal in the alternate screen. Taking a
/// writer rather than reaching for stdout is what lets a test drive this.
///
/// Raw mode is a property of a real terminal: a buffer has no line discipline to
/// switch off, so a refusal there is not a failure and the screen is drawn the same
/// way either way. Only the alternate screen itself is required, and it is restored
/// even when the keys run out.
///
/// # Errors
///
/// As [`run`], plus `Failure::execution` when the screen cannot be prepared.
pub fn run_on<W: std::io::Write>(
    screen: W,
    state: &mut FormState,
    keys: impl IntoIterator<Item = Key>,
    submit: impl FnMut(&FormState) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, Failure> {
    use crossterm::execute;
    use crossterm::terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    };

    let mut screen = screen;

    let raw = enable_raw_mode().is_ok();
    execute!(&mut screen, EnterAlternateScreen)
        .map_err(|error| Failure::execution(format!("cannot enter the screen: {error}")))?;

    // The terminal borrows the writer for this block only, so the screen can be left
    // below no matter which way the loop ends.
    let outcome = {
        let backend = CrosstermBackend::new(&mut screen);

        // A real terminal knows its size and follows it if the window changes. A
        // writer that is not a terminal — a buffer in a test, a pipe — has no size to
        // ask for, so it is drawn at a conventional one rather than refusing to run.
        let opened = match crossterm::terminal::size() {
            Ok(_size) => Terminal::new(backend),
            Err(_) => Terminal::with_options(
                backend,
                TerminalOptions {
                    viewport: Viewport::Fixed(OFF_SCREEN),
                },
            ),
        };

        match opened {
            Ok(mut terminal) => run(&mut terminal, state, keys, submit),
            Err(error) => Err(Failure::execution(format!(
                "cannot open the screen: {error}"
            ))),
        }
    };

    // Leave the screen whatever happened; the original failure is the one worth
    // reporting, so a second failure here does not replace it.
    let _ = execute!(&mut screen, LeaveAlternateScreen);
    if raw {
        let _ = disable_raw_mode();
    }

    outcome
}

/// Reads keystrokes from the process's terminal.
///
/// This is the only place that touches a real event stream, which is why every test
/// drives [`run`] with a script instead.
pub fn live_keys() -> impl Iterator<Item = Key> {
    std::iter::from_fn(|| crossterm::event::read().ok()).map_while(|event| translate(&event))
}

/// Maps a terminal event onto a key the form understands.
///
/// `None` means "not a key we care about" — a resize, a mouse report, a key release.
#[must_use]
pub fn translate(event: &crossterm::event::Event) -> Option<Key> {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    let Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        ..
    }) = *event
    else {
        return None;
    };
    if kind == KeyEventKind::Release {
        return None;
    }

    match code {
        KeyCode::Char(_) if modifiers.contains(KeyModifiers::CONTROL) => None,
        KeyCode::Char(character) => Some(Key::Char(character)),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::BackTab => Some(Key::BackTab),
        KeyCode::Enter => Some(Key::Enter),
        KeyCode::Esc => Some(Key::Escape),
        _ => None,
    }
}
