//! Runs Clifacet's reference command against the real process streams.
//!
//! Everything interesting is in the crate's library; this file is the `main` that a
//! real application would write, kept small so the application stays testable with
//! buffers instead of a terminal.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = clifacet_example::main(std::env::args_os(), &mut out, &mut err);
    // A failed flush must not report success, so it downgrades the exit code.
    if out.flush().and_then(|()| err.flush()).is_err() {
        return ExitCode::from(u8::try_from(clifacet_example::EXIT_EXECUTION_FAILED).unwrap_or(1));
    }
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
