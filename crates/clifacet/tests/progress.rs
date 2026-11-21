#![cfg(feature = "async")]
#![allow(
    clippy::pedantic,
    clippy::unnecessary_wraps,
    clippy::needless_pass_by_value,
    clippy::unused_self
)]

use std::io::Write;
use std::sync::{Arc, Mutex};

use clifacet::r#async::{NdjsonProgressSink, ProgressSink, RunCx, TtyProgressSink};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Prompter, Scripted};
use clifacet::{App, Failure, Globals, Plain, app, commands};

fn prompter() -> Box<dyn Prompter> {
    Box::new(Scripted::default())
}

#[derive(Clone)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.lock().unwrap().flush()
    }
}

#[derive(Clone, Default)]
struct DownloadCx;

fn build_cx(_cli: &DownloadCli) -> Result<DownloadCx, Failure> {
    Ok(DownloadCx)
}

#[app(
    name = "progcli",
    version = "1.0.0",
    about = "Progress test application",
    commands
)]
struct DownloadCli {
    #[command(flatten)]
    globals: Globals,

    #[clifacet(context = build_cx)]
    cx: DownloadCx,
}

#[commands]
impl DownloadCli {
    /// Simulates a download with progress updates.
    pub async fn download(&self, run: &RunCx) -> Result<String, Failure> {
        run.report_total(Some(100));
        run.message("downloading chunk 1");
        run.advance(50);
        run.message("downloading chunk 2");
        run.advance(50);
        Ok("download complete".to_string())
    }
}

#[tokio::test]
async fn test_interactive_tty_mode_displays_ephemeral_progress_on_stderr() {
    let progress_buf = Arc::new(Mutex::new(Vec::new()));
    let app = App::<DownloadCli, DownloadCli, Plain, DownloadCx>::for_cli()
        .progress_writer(SharedWriter(progress_buf.clone()));

    let mut out = Vec::new();
    let mut err = Vec::new();

    // Plain interactive TTY mode
    let caps = Capabilities::terminal();
    assert!(caps.is_terminal());

    let code = app
        .run_with_async(["progcli", "download"], caps, prompter, &mut out, &mut err)
        .await;

    assert_eq!(code, 0);

    // Stdout contains command output only
    let stdout_str = String::from_utf8(out).unwrap();
    assert!(stdout_str.contains("download complete"));
    assert!(!stdout_str.contains("downloading chunk"));

    // Ephemeral progress updates rendered to progress writer
    let prog_str = String::from_utf8(progress_buf.lock().unwrap().clone()).unwrap();
    assert!(prog_str.contains("\r\x1b[2K"));
    assert!(prog_str.contains("downloading chunk 1 [50/100] (50%)"));
    assert!(prog_str.contains("downloading chunk 2 [100/100] (100%)"));
    // Finished progress is cleared with \r\x1b[2K
    assert!(prog_str.ends_with("\r\x1b[2K"));
}

#[tokio::test]
async fn test_non_tty_mode_suppresses_all_progress_rendering() {
    let progress_buf = Arc::new(Mutex::new(Vec::new()));
    let app = App::<DownloadCli, DownloadCli, Plain, DownloadCx>::for_cli()
        .progress_writer(SharedWriter(progress_buf.clone()));

    let mut out = Vec::new();
    let mut err = Vec::new();

    // Headless / non-TTY / batch mode
    let caps = Capabilities::headless();
    assert!(!caps.is_terminal());

    let code = app
        .run_with_async(["progcli", "download"], caps, prompter, &mut out, &mut err)
        .await;

    assert_eq!(code, 0);

    // Stdout contains command output
    let stdout_str = String::from_utf8(out).unwrap();
    assert!(stdout_str.contains("download complete"));

    // Non-TTY mode completely suppresses progress rendering
    let prog_bytes = progress_buf.lock().unwrap().clone();
    assert!(
        prog_bytes.is_empty(),
        "Expected no progress output in non-TTY mode, got: {}",
        String::from_utf8_lossy(&prog_bytes)
    );
}

#[tokio::test]
async fn test_json_mode_emits_structured_ndjson_progress_to_stderr_without_contaminating_stdout() {
    let progress_buf = Arc::new(Mutex::new(Vec::new()));
    let app = App::<DownloadCli, DownloadCli, Plain, DownloadCx>::for_cli()
        .progress_writer(SharedWriter(progress_buf.clone()));

    let mut out = Vec::new();
    let mut err = Vec::new();

    // Even in terminal mode, --json emits NDJSON on stderr
    let caps = Capabilities::terminal();

    let code = app
        .run_with_async(
            ["progcli", "--json", "download"],
            caps,
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0, "err: {}", String::from_utf8_lossy(&err));

    // Stdout must contain pure command output (valid JSON data) without progress pollution
    let stdout_str = String::from_utf8(out).unwrap();
    assert!(!stdout_str.contains("event"));
    let json_val: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("stdout in --json mode must be valid JSON");
    assert_eq!(json_val, serde_json::json!("download complete"));

    // Stderr/progress stream must contain line-delimited NDJSON events
    let prog_str = String::from_utf8(progress_buf.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = prog_str.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() >= 4,
        "Expected at least 4 NDJSON progress lines, got: {prog_str}"
    );

    for line in &lines {
        let event: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: '{line}', error: {e}"));
        assert_eq!(event["event"], "progress");
    }

    // Inspect specific progress steps
    let events: Vec<serde_json::Value> = lines
        .iter()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    // Event 1: total set to 100
    assert_eq!(events[0]["total"], 100);
    assert_eq!(events[0]["current"], 0);

    // Event 2: message "downloading chunk 1"
    assert_eq!(events[1]["message"], "downloading chunk 1");
    assert_eq!(events[1]["total"], 100);

    // Event 3: advance 50 -> current 50
    assert_eq!(events[2]["current"], 50);
    assert_eq!(events[2]["total"], 100);

    // Event 4: message "downloading chunk 2"
    assert_eq!(events[3]["message"], "downloading chunk 2");
    assert_eq!(events[3]["current"], 50);

    // Event 5: advance 50 -> current 100
    assert_eq!(events[4]["current"], 100);
    assert_eq!(events[4]["total"], 100);
}

#[tokio::test]
async fn test_json_mode_subcommand_trailing_flag() {
    let progress_buf = Arc::new(Mutex::new(Vec::new()));
    let app = App::<DownloadCli, DownloadCli, Plain, DownloadCx>::for_cli()
        .progress_writer(SharedWriter(progress_buf.clone()));

    let mut out = Vec::new();
    let mut err = Vec::new();

    let code = app
        .run_with_async(
            ["progcli", "download", "--json"],
            Capabilities::terminal(),
            prompter,
            &mut out,
            &mut err,
        )
        .await;

    assert_eq!(code, 0, "err: {}", String::from_utf8_lossy(&err));
    let stdout_str = String::from_utf8(out).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout_str).unwrap();
    assert_eq!(json_val, serde_json::json!("download complete"));
}

#[cfg(feature = "tui")]
#[test]
fn test_tui_frontend_renders_live_progress_via_ratatui_gauge() {
    use clifacet::r#async::TuiProgressSink;
    use clifacet::tui::ProgressGauge;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::Widget;

    let (sink, state) = TuiProgressSink::new();

    // Initial state
    assert_eq!(state.current(), 0);
    assert_eq!(state.total(), None);
    assert_eq!(state.ratio(), None);
    assert_eq!(state.percent(), None);
    assert_eq!(state.message(), None);

    // Set total and message
    sink.total(Some(200));
    sink.message("Processing items");
    assert_eq!(state.total(), Some(200));
    assert_eq!(state.message().as_deref(), Some("Processing items"));

    // Advance to 100 (50%)
    sink.advance(100);
    assert_eq!(state.current(), 100);
    assert!((state.ratio().unwrap() - 0.5).abs() < f64::EPSILON);
    assert_eq!(state.percent(), Some(50));

    // Render into Ratatui buffer
    let area = Rect::new(0, 0, 40, 3);
    let mut buf = Buffer::empty(area);
    let gauge = ProgressGauge::from_state(&state);
    gauge.render(area, &mut buf);

    // Verify buffer contains message and percentage
    let rendered_text: String = (0..area.height)
        .map(|y| {
            let row: String = (0..area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect();
            row
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        rendered_text.contains("Processing items"),
        "Rendered buffer should contain status message: {rendered_text}"
    );
    assert!(
        rendered_text.contains("50%"),
        "Rendered buffer should contain 50%: {rendered_text}"
    );

    // Advance to 200 (100%)
    sink.advance(100);
    assert_eq!(state.current(), 200);
    assert_eq!(state.percent(), Some(100));

    let mut buf2 = Buffer::empty(area);
    let gauge2 = ProgressGauge::from_state(&state);
    gauge2.render(area, &mut buf2);

    let rendered_text2: String = (0..area.height)
        .map(|y| {
            let row: String = (0..area.width)
                .map(|x| buf2[(x, y)].symbol().to_string())
                .collect();
            row
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        rendered_text2.contains("100%"),
        "Rendered buffer should contain 100%: {rendered_text2}"
    );
}

#[test]
fn test_tty_progress_sink_formatting_variants() {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = TtyProgressSink::from_writer(SharedWriter(buf.clone()));

    // 1. Advance without total or message
    sink.advance(5);
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("[5]"));

    // 2. Message without total
    buf.lock().unwrap().clear();
    sink.message("Indexing");
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("Indexing [5]"));

    // 3. Message with total
    buf.lock().unwrap().clear();
    sink.total(Some(10));
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("Indexing [5/10] (50%)"));

    // 4. Clear
    buf.lock().unwrap().clear();
    sink.clear();
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert_eq!(s, "\r\x1b[2K");

    // 5. Total without message
    buf.lock().unwrap().clear();
    let sink2 = TtyProgressSink::from_writer(SharedWriter(buf.clone()));
    sink2.total(Some(10));
    sink2.advance(4);
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("[4/10] (40%)"));

    // 6. Total of 0 without message
    buf.lock().unwrap().clear();
    let sink3 = TtyProgressSink::from_writer(SharedWriter(buf.clone()));
    sink3.total(Some(0));
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("[0/0]"));

    // 7. Total of 0 with message
    buf.lock().unwrap().clear();
    let sink4 = TtyProgressSink::from_writer(SharedWriter(buf.clone()));
    sink4.message("Empty");
    sink4.total(Some(0));
    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(s.contains("Empty [0/0]"));
}

#[test]
fn test_run_cx_progress_delegation() {
    let (cx, rec) = RunCx::for_test();
    cx.report_total(Some(50));
    cx.message("running");
    cx.advance(25);
    assert_eq!(rec.current(), 25);
    assert_eq!(rec.total(), Some(50));
    assert_eq!(rec.last_message().as_deref(), Some("running"));
    let _p = cx.progress_arc();
    let _pr = cx.progress();
}

#[test]
fn test_ndjson_progress_sink_indeterminate() {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let sink = NdjsonProgressSink::from_writer(SharedWriter(buf.clone()));

    sink.message("Scanning");
    sink.advance(42);

    let s = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = s.lines().collect();
    assert_eq!(lines.len(), 2);

    let v1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v1["event"], "progress");
    assert_eq!(v1["message"], "Scanning");
    assert!(v1["total"].is_null());
    assert_eq!(v1["current"], 0);

    let v2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(v2["current"], 42);
    assert!(v2["total"].is_null());
}

#[cfg(feature = "tui")]
#[test]
fn test_progress_gauge_explicit_constructor_and_styling() {
    use clifacet::tui::ProgressGauge;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Style};
    use ratatui::widgets::Widget;

    let area = Rect::new(0, 0, 30, 1);

    // 1. With message and total
    let mut buf = Buffer::empty(area);
    let gauge =
        ProgressGauge::new(5, Some(10), Some("Syncing")).style(Style::default().fg(Color::Yellow));
    gauge.render(area, &mut buf);
    let s = (0..area.width)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<String>();
    assert!(s.contains("Syncing"));
    assert!(s.contains("50%"));

    // 2. Without message, with total
    let mut buf = Buffer::empty(area);
    let gauge = ProgressGauge::new(3, Some(10), None);
    gauge.render(area, &mut buf);
    let s = (0..area.width)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<String>();
    assert!(s.contains("30%"));

    // 3. Indeterminate (total = None) with message
    let mut buf = Buffer::empty(area);
    let gauge = ProgressGauge::new(42, None, Some("Polling"));
    gauge.render(area, &mut buf);
    let s = (0..area.width)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<String>();
    assert!(s.contains("Polling [42]"));

    // 4. Indeterminate (total = None) without message
    let mut buf = Buffer::empty(area);
    let gauge = ProgressGauge::new(7, None, None);
    gauge.render(area, &mut buf);
    let s = (0..area.width)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<String>();
    assert!(s.contains("[7]"));

    // 5. Total = Some(0)
    let mut buf = Buffer::empty(area);
    let gauge = ProgressGauge::new(0, Some(0), Some("Empty"));
    gauge.render(area, &mut buf);
    let s = (0..area.width)
        .map(|x| buf[(x, 0)].symbol())
        .collect::<String>();
    assert!(s.contains("Empty [0]"));
}

#[test]
fn test_progress_sink_debug_representations() {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let tty = TtyProgressSink::from_writer(SharedWriter(buf.clone()));
    let dbg_tty = format!("{tty:?}");
    assert!(dbg_tty.contains("TtyProgressSink"));

    let ndjson = NdjsonProgressSink::from_writer(SharedWriter(buf.clone()));
    let dbg_ndjson = format!("{ndjson:?}");
    assert!(dbg_ndjson.contains("NdjsonProgressSink"));

    #[cfg(feature = "tui")]
    {
        use clifacet::r#async::TuiProgressSink;
        let (tui_sink, _) = TuiProgressSink::new();
        let _st = tui_sink.state();
        let dbg_tui = format!("{tui_sink:?}");
        assert!(dbg_tui.contains("TuiProgressSink"));
    }
}
