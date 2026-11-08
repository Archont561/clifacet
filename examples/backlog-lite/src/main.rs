//! Runs the Backlog.md wrapper: one call, because the runner owns the pipeline.
//!
//! Everything worth testing lives in the library; this file is the `main` a real
//! application writes now that clifacet's `App` parses, asks, validates, previews, runs,
//! renders and maps the exit code (CLI-46, CLI-48).

fn main() -> std::process::ExitCode {
    clifacet_backlog_lite::run_app()
}
