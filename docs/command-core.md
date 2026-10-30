# Command core and its seams

This is the M1 slice: `clap` parses, the application validates, a UI-free command
runs, and the application decides which stream carries the data and what a
failure costs. `crates/clifacet/src/example.rs` is the whole thing in one place,
and `crates/clifacet/tests/example_command.rs` drives it through buffers rather
than a terminal.

## The four layers

| Layer | Lives in | Owns | Must not |
| --- | --- | --- | --- |
| `clap` surface | `example::Cli` | flags, help, usage errors | know about the command |
| Validation | `TryFrom<Cli> for ExampleInput` | rejecting bad values, defaults | run anything |
| Command | `impl Command for Example` | the operation | touch a stream, read a terminal, know `clap` exists |
| Application | `example::run`, `example::main` | streams, exit codes, diagnostics | contain the operation |

`Command` is the only seam between the middle two rows, and it is the only one
that has to stay stable for later frontends:

```rust
pub trait Command {
    type Input;
    type Output;
    fn execute(&self, input: Self::Input) -> Result<Self::Output, Failure>;
}
```

`Input` is constructed *after* validation, so every value of it is runnable and
`execute` has nothing left to reject. `Output` is data, not a message: the
application decides whether it becomes stdout, JSON or a table.

## Ownership, per decision-3

**Failures are library-owned; exit codes are not.** `Failure` answers "what kind
of failure is this" — `Validation` (nothing ran) or `Execution` (it ran and could
not finish) — because every frontend must report those the same way. The numbers
are the application's: this repository's example uses `0` success, `1` execution,
`2` validation and usage. A different application may choose otherwise without
touching the library.

**Streams are the application's.** `Command` never writes. `example::run` takes
`out` and `err` as parameters, which is what makes a run testable without a
terminal and is also why "stdout carries only requested machine data" is a
property a test can assert rather than a convention.

## Demonstrated behavior

Measured on `crates/clifacet` at this commit:

| Invocation | Exit | stdout | stderr |
| --- | --- | --- | --- |
| `example` | 0 | `hello, world` | empty |
| `example --repeat 3` | 0 | three lines | empty |
| `example --repeat 0` | 2 | empty | `example: --repeat must be between 1 and 8, got 0` |
| `example --explode` | 1 | empty | `example: the example operation failed on purpose (--explode)` |
| `example --nope` | 2 | empty | `clap` usage error |
| `example --help` | 0 | help text | empty |
| closed data stream | 1 | — | `example: cannot write the output: …` |

Two details worth keeping:

- **A range check is not a parser's job.** `--repeat 255` parses as a `u8`, so the
  bound lives in validation and is tested there.
- **Help and `--version` are success on the data stream**, because they are the
  result the caller asked for. `clap` decides that with `Error::use_stderr`, and
  the application follows it instead of guessing.

## Known unsupported behavior

Documented so nobody reads absence as oversight:

- **Interactive and TUI frontends.** The `interactive` and `tui` features reserve
  dependency boundaries only; no code constructs a prompter or a terminal.
- **`--json`.** The `json` feature reserves `serde_json`; no flag selects it, so
  only the human data line exists.
- **Runtime mode resolution.** Explicit request, application policy and capability
  detection (decision-3's precedence) arrive with M2. Until then there is exactly
  one mode: flags in, data out.
- **Cancellation.** Interactive cancellation needs its own category, so
  `FailureKind` is `#[non_exhaustive]` and adding one is not a break.
- **The companion derive.** [`TryFrom<Cli>`] is written by hand here; CLI-7 adds
  the derive and the `trybuild` cases. Canonical argv replay and any construct
  `clap` cannot re-validate stay out of scope until that task proves them.
- **Secret handling.** No input is marked secret, so nothing redacts. CLI-14 owns
  the contract, including what may appear in previews and logs.
