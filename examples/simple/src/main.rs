//! Runs Clifacet's reference command against the real process streams.
//!
//! Everything interesting is in the crate's library: [`clifacet_example::app`] is the
//! whole application as one `clifacet::App`, and this `main` runs it — parse, ask,
//! validate, run, write the output, exit.

fn main() -> std::process::ExitCode {
    clifacet_example::app().run()
}
