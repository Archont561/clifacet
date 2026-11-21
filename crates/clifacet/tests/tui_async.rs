//! Tests for TUI EventStream loop and TerminalGuard (CLI-85, decision-26).

#![cfg(all(feature = "tui", feature = "async"))]
#![allow(
    clippy::pedantic,
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value
)]

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use clifacet::r#async::{Cancel, ProgressSink, TuiProgressSink};
use clifacet::tui::app::{TerminalGuard, run_task_on_with, run_task_with, run_task_with_handler};
use clifacet::tui::ratatui::Terminal;
use clifacet::tui::ratatui::backend::TestBackend;
use clifacet::tui::widgets::ProgressGauge;
use clifacet::{Failure, FailureKind};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use futures_core::Stream;

/// A mock event stream that yields predefined crossterm events before staying pending.
struct MockEventStream {
    events: Vec<Event>,
    index: usize,
}

impl MockEventStream {
    fn new(events: Vec<Event>) -> Self {
        Self { events, index: 0 }
    }
}

impl Stream for MockEventStream {
    type Item = std::io::Result<Event>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.index < self.events.len() {
            let ev = self.events[self.index].clone();
            self.index += 1;
            Poll::Ready(Some(Ok(ev)))
        } else {
            Poll::Pending
        }
    }
}

/// An async task that yields several times, updating progress on each turn.
struct SteppingTask {
    steps_left: usize,
    sink: TuiProgressSink,
}

impl std::future::Future for SteppingTask {
    type Output = Result<String, Failure>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.steps_left == 0 {
            Poll::Ready(Ok("all work completed".to_string()))
        } else {
            self.steps_left -= 1;
            self.sink.advance(1);
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

#[tokio::test]
async fn test_tui_loop_redrawing_while_background_task_executes() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");
    let (sink, progress) = TuiProgressSink::new();
    sink.total(Some(5));
    sink.message("Deploying microservices");

    let draw_count = Arc::new(AtomicUsize::new(0));
    let draw_count_clone = Arc::clone(&draw_count);
    let progress_clone = Arc::clone(&progress);

    let task = SteppingTask {
        steps_left: 5,
        sink,
    };

    let events = MockEventStream::new(vec![]);
    let cancel = Cancel::new();

    let res = run_task_with(&mut terminal, events, task, Some(cancel), move |frame| {
        draw_count_clone.fetch_add(1, Ordering::SeqCst);
        let gauge = ProgressGauge::from_state(&progress_clone);
        frame.render_widget(gauge, frame.area());
    })
    .await;

    assert_eq!(res.unwrap(), "all work completed");
    // Verify that multiple redraws occurred while the background task executed
    assert!(
        draw_count.load(Ordering::SeqCst) >= 5,
        "expected at least 5 redraws during task execution"
    );

    // Verify terminal buffer reflects the rendered gauge
    let buffer = terminal.backend().buffer().clone();
    let text = (0..buffer.area.height)
        .map(|line| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, line)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(text.contains("Deploying microservices"));
    assert!(text.contains("100%"));
}

#[tokio::test]
async fn test_tui_loop_cancellation_via_escape_key() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");
    let cancel = Cancel::new();

    let esc_event = Event::Key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let events = MockEventStream::new(vec![esc_event]);

    // An unresolved background task
    let pending_task = std::future::pending::<Result<String, Failure>>();

    let res = run_task_with(
        &mut terminal,
        events,
        pending_task,
        Some(cancel.clone()),
        |_frame| {},
    )
    .await;

    let err = res.expect_err("should cancel");
    assert_eq!(err.kind(), FailureKind::Cancelled);
    assert!(cancel.is_cancelled());
    assert!(err.to_string().contains("cancelled by the user"));
}

#[tokio::test]
async fn test_tui_loop_cancellation_via_ctrl_c() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");
    let cancel = Cancel::new();

    let ctrl_c_event = Event::Key(KeyEvent {
        code: KeyCode::Char('c'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let events = MockEventStream::new(vec![ctrl_c_event]);

    let pending_task = std::future::pending::<Result<String, Failure>>();

    let res = run_task_with(
        &mut terminal,
        events,
        pending_task,
        Some(cancel.clone()),
        |_frame| {},
    )
    .await;

    let err = res.expect_err("should cancel on Ctrl-C");
    assert_eq!(err.kind(), FailureKind::Cancelled);
    assert!(cancel.is_cancelled());
}

#[tokio::test]
async fn test_tui_loop_custom_event_handler() {
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("test backend");

    let enter_event = Event::Key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let events = MockEventStream::new(vec![enter_event]);

    let pending_task = std::future::pending::<Result<String, Failure>>();

    let res = run_task_with_handler(
        &mut terminal,
        events,
        pending_task,
        None,
        |_frame| {},
        |event| {
            if let Event::Key(KeyEvent {
                code: KeyCode::Enter,
                ..
            }) = event
            {
                Some(Ok("intercepted enter".to_string()))
            } else {
                None
            }
        },
    )
    .await;

    assert_eq!(res.unwrap(), "intercepted enter");
}

#[test]
fn test_terminal_guard_restores_on_normal_drop() {
    let mut buffer = Vec::new();
    {
        let _guard = TerminalGuard::enter(&mut buffer).expect("enter alternate screen");
    }
    let output = String::from_utf8_lossy(&buffer);
    // EnterAlternateScreen: CSI ? 1049 h, LeaveAlternateScreen: CSI ? 1049 l
    assert!(
        output.contains("\x1b[?1049h"),
        "should enter alternate screen"
    );
    assert!(
        output.contains("\x1b[?1049l"),
        "should leave alternate screen on drop"
    );
}

#[test]
fn test_terminal_guard_restores_on_panic() {
    let mut buffer = Vec::new();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = TerminalGuard::enter(&mut buffer).expect("enter alternate screen");
        panic!("simulated panic inside active TUI session");
    }));

    assert!(res.is_err(), "panic should have been caught");
    let output = String::from_utf8_lossy(&buffer);
    assert!(
        output.contains("\x1b[?1049l"),
        "drop guard must emit LeaveAlternateScreen even when unwinding on panic"
    );
}

#[tokio::test]
async fn test_run_task_on_with_restores_terminal_screen_on_cancel() {
    let mut buffer = Vec::new();
    let esc_event = Event::Key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    let events = MockEventStream::new(vec![esc_event]);
    let pending_task = std::future::pending::<Result<String, Failure>>();

    let res = run_task_on_with(&mut buffer, events, pending_task, None, |_frame| {}).await;

    assert_eq!(res.unwrap_err().kind(), FailureKind::Cancelled);
    let output = String::from_utf8_lossy(&buffer);
    assert!(output.contains("\x1b[?1049h"), "entered alternate screen");
    assert!(
        output.contains("\x1b[?1049l"),
        "left alternate screen after cancellation"
    );
}
