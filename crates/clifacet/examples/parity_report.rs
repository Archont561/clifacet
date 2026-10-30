//! A parity report an application can run, built only from the published API.
//!
//! This file is the "external consumer" CLI-13 asks for: it lives outside the crate,
//! so it can see nothing but `clifacet`'s public surface. If the harness needed an
//! internal hook, this would not compile.
//!
//! ```sh
//! pixi run --frozen -x cargo run --features tui --example parity_report
//! ```
//!
//! It runs the example command three ways — flags, a scripted prompt and the
//! headless form — compares the exit code, the data, the diagnostic and the input
//! each execution received, and exits non-zero if any mode disagrees.

use clifacet::example::{self, Example, ExampleInput};
use clifacet::harness::{Outcome, Parity, Recording, Subject};
use clifacet::policy::Capabilities;
use clifacet::prompt::{Answer, Scripted};
use clifacet::tui::Key;

/// The answer every mode is asked to supply.
const NAME: &str = "ada";

/// How a recorded input is rendered for the report.
fn describe(input: &ExampleInput) -> String {
    format!("{input:?}")
}

/// Runs the command as if `--name <NAME>` had been typed.
fn flags() -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let argv = [
        String::from("example"),
        String::from("--name"),
        String::from(NAME),
    ];

    Outcome::measured(|out, err| {
        example::main_with_command(
            &command,
            argv,
            Capabilities::headless(),
            || Box::new(Scripted::default()),
            out,
            err,
        )
    })
    .with_effects(effects.recorded())
}

/// Runs the command with someone answering `NAME` when asked.
fn prompted() -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let script = Scripted::new([Answer::Text(String::from(NAME))]);

    Outcome::measured(|out, err| {
        example::main_with_command(
            &command,
            [String::from("example")],
            Capabilities::terminal(),
            || Box::new(script.clone()),
            out,
            err,
        )
    })
    .with_effects(effects.recorded())
}

/// Runs the command through the form, typing `NAME` and submitting.
fn form() -> Outcome {
    let (command, effects) = Recording::new(Example, describe as fn(&ExampleInput) -> String);
    let keys: Vec<Key> = NAME
        .chars()
        .map(Key::Char)
        .chain(std::iter::once(Key::Enter))
        .collect();

    Outcome::measured(|out, err| example::run_tui_with(&command, Vec::new(), keys, out, err))
        .with_effects(effects.recorded())
}

fn main() {
    let runs = [
        ("flags", flags()),
        ("scripted prompt", prompted()),
        ("headless form", form()),
    ];

    println!(
        "{:<16} {:>4}  {:<16} recorded input",
        "mode", "exit", "data"
    );
    for (label, outcome) in &runs {
        println!(
            "{label:<16} {:>4}  {:<16} {}",
            outcome.exit(),
            format!("{:?}", outcome.data()),
            outcome.effects().join("; ")
        );
    }

    let Some(((reference_label, reference), others)) = runs.split_first() else {
        return;
    };

    let parity = others.iter().fold(
        Parity::new(*reference_label, reference.clone()).comparing([
            Subject::Exit,
            Subject::Data,
            Subject::Diagnostic,
            Subject::Effects,
        ]),
        |parity, (label, outcome)| parity.compare(*label, outcome.clone()),
    );

    match parity.check() {
        Ok(()) => println!("\nall three modes agree"),
        Err(differences) => {
            eprintln!("\nthe modes disagree:");
            for difference in differences {
                eprintln!("  {difference}");
            }
            std::process::exit(1);
        }
    }
}
