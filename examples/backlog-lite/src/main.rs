//! Runs the Backlog.md wrapper against the real process streams.
//!
//! Everything worth testing lives in the library; this file is the `main` a real
//! application would write.

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut out = std::io::stdout().lock();
    let mut err = std::io::stderr().lock();
    let code = clifacet_backlog_lite::main(std::env::args_os(), &mut out, &mut err);
    if out.flush().and_then(|()| err.flush()).is_err() {
        return ExitCode::from(u8::try_from(clifacet_backlog_lite::EXIT.execution()).unwrap_or(1));
    }
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
