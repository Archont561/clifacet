# Getting started

Clifacet keeps one command core and renders it through four frontends: flags, a
prompt, a full-screen form, and whatever the application draws itself. The command
never writes a stream and never reads a terminal — that is what makes the same
command reachable from all of them, and testable without any of them.

## Add the dependency

```toml
[dependencies]
clifacet = { version = "0.2.0", features = ["tui"] }
clap = "4.6.7"
```

### Features

Every feature is off by default, and each one is a dependency boundary: a feature you
do not enable is not compiled, and neither are the crates it would bring in.

| Feature | What it adds | What it costs |
| --- | --- | --- |
| *(none)* | commands, the derive, the policy, the plain frontend, secrets, provenance and the harness | `clap`, `thiserror` and the `clifacet-macros` proc macro |
| `interactive` | the `inquire`-backed dialog behind `Prompter`, so prompts work at a terminal | `inquire` with its `crossterm` backend — the largest prompt-side tree |
| `tui` | the full-screen form in `clifacet::tui` and the TUI mode in the policy | `ratatui` and `crossterm` — the largest tree of the three |
| `serde` | `Serialize` for `Failure` and `FailureKind` (`{"kind":"validation","message":"…"}`) and for `Secret`, which serialises as `[redacted]` | `serde` and its derive macro, a proc macro built at compile time |

`serde` is how an application reports failures as JSON: it serialises a `Failure` with
`serde_json` under a flag of its own, on a stream of its own, because output is the
application's (decision-3) and the library writes no JSON itself. There is no `json`
feature — 0.2.0 reserved one that selected `serde_json` and did nothing else; it was
retired in favour of `serde`, so an application that wants JSON depends on `serde_json`
directly and enables `serde` here.

Every combination is checked in CI — see
[Supported platforms and feature combinations](development.md#supported-platforms-and-feature-combinations).

## Write the command

A `Command` is a value that takes validated input and returns data or a failure. It
is the only seam the frontends share, so it stays deliberately small:

```rust
use clifacet::{Command, Failure};

struct Greet;

impl Command for Greet {
    type Input = String;
    type Output = String;

    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure> {
        Ok(format!("hello, {input}"))
    }
}

let greeting = Greet.execute("ada".to_owned()).expect("a greeting");
assert_eq!(greeting, "hello, ada");
```

`Input` is built *after* validation, so `execute` has nothing left to reject, and
`Output` is data rather than a message: whether it becomes a line of text, JSON or a
table is the application's decision.

## Run it

Parsing, validation, the streams and the exit code belong to the application. The
reference application in `examples/simple` shows the whole path, and is the
shortcut to seeing a mode work:

```sh
cargo run -p clifacet-example                            # asks for what the flags left out
cargo run -p clifacet-example -- --name ada --repeat 2    # flags only
cargo run -p clifacet-example -- --interactive            # the dialog, explicitly
cargo run -p clifacet-example -- --tui                    # the full-screen form
```

Its own [README](../examples/simple/README.md) is the guided tour: the flags, the four
modes, the exit codes and which test proves each promise.

Its library is the shape to copy: a `clap` struct, a `TryFrom` bridge to validated
input, the command, and an application layer that owns both streams and the exit
codes.

## What to read next

| Question | Page |
| --- | --- |
| Which mode runs, and who decides? | [Modes and the interaction policy](modes.md) |
| How does an existing `clap` command get validated input? | [The companion derive](derive.md) |
| How do I draw my own screen without a second command? | [TUI composition](tui-composition.md) |
| How is any of this tested without a terminal? | [Testing](testing.md) |
| How do I work on the crate itself? | [Contributing](contributing.md) |

The reference material — every seam, the ownership rules and what is deliberately
not implemented — is in [Command core and its seams](command-core.md).
