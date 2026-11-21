//! The loop that draws the form, reads keys and submits.
//!
//! It takes a terminal and an iterator of [`Key`]s, so the same code runs a real
//! session and a scripted one. The screen it draws on is `ratatui`'s; the keys are
//! this crate's; the submission is the caller's — which is what keeps the TUI from
//! owning validation or the handler.

use std::io::IsTerminal as _;

use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::buffer::Buffer;
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
    submit: impl FnMut(&FormState) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, Failure> {
    run_with(terminal, state, keys, draw_form, submit)
}

/// Draws the whole screen with [`Form`].
pub fn draw_form(state: &FormState, area: Rect, buffer: &mut Buffer) {
    Form::new(state).render(area, buffer);
}

/// Draws the form the application wants, and submits it like any other.
///
/// This is the second of decision-4's two escape hatches, and it changes exactly one
/// thing: `draw` replaces the drawing. The keys, the focus, the refusals and the
/// submission are the same loop, so a screen of the application's own reaches the
/// same validation and the same handler as the generated form — there is no second
/// command to keep in step.
///
/// # Errors
///
/// As [`run`].
pub fn run_with<B: Backend>(
    terminal: &mut Terminal<B>,
    state: &mut FormState,
    keys: impl IntoIterator<Item = Key>,
    draw: impl Fn(&FormState, Rect, &mut Buffer),
    mut submit: impl FnMut(&FormState) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, Failure> {
    let mut keys = keys.into_iter();

    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();
                draw(state, area, frame.buffer_mut());
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
    run_on_with(screen, state, keys, draw_form, submit)
}

/// A drop guard that restores the terminal to cooked mode and leaves the alternate screen
/// on drop, guaranteeing cleanup even if a panic or hard cancellation occurs.
pub struct TerminalGuard<W: std::io::Write> {
    writer: Option<W>,
    raw_mode_enabled: bool,
    alternate_screen_entered: bool,
}

impl<W: std::io::Write> TerminalGuard<W> {
    /// Enters the alternate screen and enables terminal raw mode on `writer`.
    ///
    /// # Errors
    ///
    /// Returns [`Failure::execution`] if entering the alternate screen fails.
    pub fn enter(mut writer: W) -> Result<Self, Failure> {
        let raw = crossterm::terminal::enable_raw_mode().is_ok();
        if let Err(error) =
            crossterm::execute!(&mut writer, crossterm::terminal::EnterAlternateScreen)
        {
            if raw {
                let _ = crossterm::terminal::disable_raw_mode();
            }
            return Err(Failure::execution(format!(
                "cannot enter the screen: {error}"
            )));
        }
        Ok(Self {
            writer: Some(writer),
            raw_mode_enabled: raw,
            alternate_screen_entered: true,
        })
    }

    /// Accesses the underlying writer mutably.
    ///
    /// # Panics
    ///
    /// Panics if the guard was already consumed.
    pub fn writer_mut(&mut self) -> &mut W {
        self.writer
            .as_mut()
            .expect("terminal writer is present until drop")
    }

    /// Restores the terminal explicitly, leaving the alternate screen and disabling raw mode.
    pub fn restore(&mut self) {
        self.clean_up();
    }

    /// Consumes the guard, restoring the terminal and returning the inner writer.
    ///
    /// # Panics
    ///
    /// Panics if the guard was already consumed.
    pub fn into_inner(mut self) -> W {
        self.clean_up();
        self.writer
            .take()
            .expect("terminal writer is present until drop")
    }

    fn clean_up(&mut self) {
        if self.alternate_screen_entered {
            if let Some(writer) = self.writer.as_mut() {
                let _ = crossterm::execute!(writer, crossterm::terminal::LeaveAlternateScreen);
            }
            self.alternate_screen_entered = false;
        }
        if self.raw_mode_enabled {
            let _ = crossterm::terminal::disable_raw_mode();
            self.raw_mode_enabled = false;
        }
    }
}

impl<W: std::io::Write> Drop for TerminalGuard<W> {
    fn drop(&mut self) {
        self.clean_up();
    }
}

impl<W: std::io::Write> std::ops::Deref for TerminalGuard<W> {
    type Target = W;

    fn deref(&self) -> &Self::Target {
        self.writer
            .as_ref()
            .expect("terminal writer is present until drop")
    }
}

impl<W: std::io::Write> std::ops::DerefMut for TerminalGuard<W> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.writer
            .as_mut()
            .expect("terminal writer is present until drop")
    }
}

/// The alternate-screen run, drawing whatever the application asks for.
///
/// Split from [`run_on`] only by the drawing, so a custom screen inherits the same
/// promise: the screen is entered and left on every path, cancellation included.
///
/// # Errors
///
/// As [`run_on`].
pub fn run_on_with<W: std::io::Write>(
    screen: W,
    state: &mut FormState,
    keys: impl IntoIterator<Item = Key>,
    draw: impl Fn(&FormState, Rect, &mut Buffer),
    submit: impl FnMut(&FormState) -> Result<Vec<String>, String>,
) -> Result<Vec<String>, Failure> {
    let mut guard = TerminalGuard::enter(screen)?;

    // The terminal borrows the writer for this block only, so the screen can be left
    // below no matter which way the loop ends.
    let outcome = {
        let backend = CrosstermBackend::new(&mut *guard);

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
            Ok(mut terminal) => run_with(&mut terminal, state, keys, draw, submit),
            Err(error) => Err(Failure::execution(format!(
                "cannot open the screen: {error}"
            ))),
        }
    };

    guard.restore();
    outcome
}

#[cfg(all(feature = "tui", feature = "async"))]
fn is_cancel_event(event: &crossterm::event::Event) -> bool {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
    if let Event::Key(KeyEvent {
        code,
        modifiers,
        kind,
        ..
    }) = *event
    {
        if kind == KeyEventKind::Release {
            return false;
        }
        if code == KeyCode::Esc {
            return true;
        }
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            return true;
        }
    }
    false
}

#[cfg(all(feature = "tui", feature = "async"))]
struct TaskEventLoop<'a, B: Backend, S, Fut, D, H> {
    terminal: &'a mut Terminal<B>,
    events: S,
    task: std::pin::Pin<&'a mut Fut>,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
    handle_event: H,
    needs_redraw: bool,
}

#[cfg(all(feature = "tui", feature = "async"))]
impl<B, S, Fut, D, H, T> std::future::Future for TaskEventLoop<'_, B, S, Fut, D, H>
where
    B: Backend,
    S: futures_core::Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin,
    Fut: std::future::Future<Output = Result<T, Failure>>,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
    H: FnMut(&crossterm::event::Event) -> Option<Result<T, Failure>> + Unpin,
{
    type Output = Result<T, Failure>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();

        // 1. Poll the background task.
        if let std::task::Poll::Ready(output) = this.task.as_mut().poll(cx) {
            let _ = this.terminal.draw(&mut this.draw);
            return std::task::Poll::Ready(output);
        }

        // 2. Check cancellation token if present.
        if let Some(ref cancel) = this.cancel
            && cancel.is_cancelled()
        {
            return std::task::Poll::Ready(Err(Failure::cancelled("the task was cancelled")));
        }

        // 3. Poll available events from the event stream.
        let mut got_event = false;
        while let std::task::Poll::Ready(Some(event_res)) =
            std::pin::Pin::new(&mut this.events).poll_next(cx)
        {
            got_event = true;
            if let Ok(event) = event_res {
                if let Some(res) = (this.handle_event)(&event) {
                    return std::task::Poll::Ready(res);
                }
                if is_cancel_event(&event) {
                    if let Some(ref cancel) = this.cancel {
                        cancel.cancel();
                    }
                    return std::task::Poll::Ready(Err(Failure::cancelled(
                        "the task was cancelled by the user",
                    )));
                }
            }
        }

        // 4. Redraw whenever the task was polled (to update progress) or an event arrived.
        if this.needs_redraw || got_event {
            this.needs_redraw = false;
            if let Err(error) = this.terminal.draw(&mut this.draw) {
                return std::task::Poll::Ready(Err(Failure::execution(format!(
                    "cannot draw to the terminal: {error}"
                ))));
            }
        }

        // Always request another redraw on next wake.
        this.needs_redraw = true;

        std::task::Poll::Pending
    }
}

/// Runs a background task in the TUI, selecting over terminal events and redrawing live.
///
/// Keystrokes are read from crossterm's live [`EventStream`](crossterm::event::EventStream).
/// An `Escape` or `Ctrl-C` key cancels the task via the provided [`Cancel`](crate::async::Cancel) token.
///
/// # Errors
///
/// Returns [`Failure::cancelled`] if the user cancels with Escape or Ctrl-C,
/// or any failure produced by `task` or terminal drawing.
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task<B: Backend, T, Fut, D>(
    terminal: &mut Terminal<B>,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
{
    run_task_with(
        terminal,
        crossterm::event::EventStream::new(),
        task,
        cancel,
        draw,
    )
    .await
}

/// Runs a background task with a custom event stream, selecting between events and task completion.
///
/// This allows testing with synthetic event streams or non-default event sources.
///
/// # Errors
///
/// As [`run_task`].
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task_with<B: Backend, T, Fut, S, D>(
    terminal: &mut Terminal<B>,
    events: S,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    S: futures_core::Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
{
    run_task_with_handler(terminal, events, task, cancel, draw, |_| None).await
}

/// Runs a background task with a custom event stream and a custom event handler.
///
/// If `handle_event` returns `Some(outcome)`, the loop returns immediately with that outcome.
///
/// # Errors
///
/// As [`run_task`].
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task_with_handler<B: Backend, T, Fut, S, D, H>(
    terminal: &mut Terminal<B>,
    events: S,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
    handle_event: H,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    S: futures_core::Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
    H: FnMut(&crossterm::event::Event) -> Option<Result<T, Failure>> + Unpin,
{
    let mut task = std::pin::pin!(task);
    let loop_fut = TaskEventLoop {
        terminal,
        events,
        task: task.as_mut(),
        cancel,
        draw,
        handle_event,
        needs_redraw: true,
    };
    loop_fut.await
}

/// Runs a background task on a writer inside the alternate screen, with terminal drop guards.
///
/// # Errors
///
/// As [`run_task`], plus [`Failure::execution`] if entering the alternate screen fails.
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task_on<W: std::io::Write, T, Fut, D>(
    screen: W,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
{
    run_task_on_with(
        screen,
        crossterm::event::EventStream::new(),
        task,
        cancel,
        draw,
    )
    .await
}

/// Runs a background task on a writer with a custom event stream and terminal drop guards.
///
/// # Errors
///
/// As [`run_task_on`].
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task_on_with<W: std::io::Write, T, Fut, S, D>(
    screen: W,
    events: S,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    S: futures_core::Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
{
    run_task_on_with_handler(screen, events, task, cancel, draw, |_| None).await
}

/// Runs a background task on a writer with custom events and custom event handler, with terminal drop guards.
///
/// # Errors
///
/// As [`run_task_on`].
#[cfg(all(feature = "tui", feature = "async"))]
pub async fn run_task_on_with_handler<W: std::io::Write, T, Fut, S, D, H>(
    screen: W,
    events: S,
    task: Fut,
    cancel: Option<crate::r#async::Cancel>,
    draw: D,
    handle_event: H,
) -> Result<T, Failure>
where
    Fut: std::future::Future<Output = Result<T, Failure>>,
    S: futures_core::Stream<Item = std::io::Result<crossterm::event::Event>> + Unpin,
    D: FnMut(&mut ratatui::Frame<'_>) + Unpin,
    H: FnMut(&crossterm::event::Event) -> Option<Result<T, Failure>> + Unpin,
{
    let mut guard = TerminalGuard::enter(screen)?;
    let backend = CrosstermBackend::new(&mut *guard);
    let opened = match crossterm::terminal::size() {
        Ok(_size) => Terminal::new(backend),
        Err(_) => Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Fixed(OFF_SCREEN),
            },
        ),
    };
    let mut terminal =
        opened.map_err(|error| Failure::execution(format!("cannot open the screen: {error}")))?;

    let outcome =
        run_task_with_handler(&mut terminal, events, task, cancel, draw, handle_event).await;
    drop(terminal);
    guard.restore();
    outcome
}

/// Reads keystrokes from the process's terminal.
///
/// This is the only place that touches a real event stream, which is why every test
/// drives [`run`] with a script instead.
pub fn live_keys() -> impl Iterator<Item = Key> {
    keys_when_interactive(std::io::stdin().is_terminal())
}

/// The rule [`live_keys`] follows, with the decision passed in so it can be tested.
///
/// Without a keyboard there is nothing to read, and `event::read()` does not always
/// say so: a process whose stdin is a pipe gets an error, but a Windows console handle
/// that exists and is not interactive blocks forever. Ending the input instead makes
/// [`run_with`] report a cancellation, which is what a form nobody can type into
/// should do.
pub fn keys_when_interactive(interactive: bool) -> impl Iterator<Item = Key> {
    std::iter::from_fn(move || interactive.then(|| crossterm::event::read().ok()).flatten())
        .map_while(|event| translate(&event))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn terminal_guard_methods_and_deref() {
        let mut buffer = Vec::new();
        let mut guard = TerminalGuard::enter(&mut buffer).expect("enter alternate screen");
        guard.writer_mut().write_all(b"test").unwrap();
        guard.write_all(b" more").unwrap();
        let deref_len = (*guard).len();
        assert!(deref_len > 0);
        guard.restore();
    }

    #[test]
    fn terminal_guard_into_inner() {
        let mut buffer = Vec::new();
        let guard = TerminalGuard::enter(&mut buffer).expect("enter alternate screen");
        let inner = guard.into_inner();
        assert_ne!(inner.as_slice(), &[] as &[u8]);
    }

    #[cfg(feature = "async")]
    #[test]
    fn test_is_cancel_event() {
        use crossterm::event::{
            Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
        };

        let esc = Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        assert!(is_cancel_event(&esc));

        let esc_release = Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });
        assert!(!is_cancel_event(&esc_release));

        let ctrl_c = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        assert!(is_cancel_event(&ctrl_c));

        let regular_key = Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        });
        assert!(!is_cancel_event(&regular_key));
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_run_task_on_with_handler_custom() {
        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct EmptyStream;
        impl Stream for EmptyStream {
            type Item = std::io::Result<crossterm::event::Event>;
            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let mut buffer = Vec::new();
        let res = run_task_on_with_handler(
            &mut buffer,
            EmptyStream,
            async { Ok("done".to_string()) },
            None,
            |_frame| {},
            |_event| None,
        )
        .await;

        assert_eq!(res.unwrap(), "done");
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_run_task_with_pre_cancelled_token() {
        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct EmptyStream;
        impl Stream for EmptyStream {
            type Item = std::io::Result<crossterm::event::Event>;
            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(80, 24)).unwrap();
        let cancel = crate::r#async::Cancel::new();
        cancel.cancel();

        let pending_task = std::future::pending::<Result<String, Failure>>();
        let res = run_task_with(
            &mut terminal,
            EmptyStream,
            pending_task,
            Some(cancel),
            |_| {},
        )
        .await;
        assert_eq!(res.unwrap_err().kind(), crate::FailureKind::Cancelled);
    }

    #[cfg(feature = "async")]
    #[tokio::test]
    async fn test_run_task_on_with_success() {
        use futures_core::Stream;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        struct EmptyStream;
        impl Stream for EmptyStream {
            type Item = std::io::Result<crossterm::event::Event>;
            fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
                Poll::Pending
            }
        }

        let mut buffer = Vec::new();
        let res = run_task_on_with(
            &mut buffer,
            EmptyStream,
            async { Ok::<_, Failure>("success_output") },
            None,
            |_frame| {},
        )
        .await;
        assert_eq!(res.unwrap(), "success_output");
        let output = String::from_utf8_lossy(&buffer);
        assert!(output.contains("\x1b[?1049h"));
        assert!(output.contains("\x1b[?1049l"));
    }
}
