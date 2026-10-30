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
| Validation | `TryFrom<Cli> for ExampleInput`, reached through `Facet::validate` | rejecting bad values, defaults | run anything |
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

## The companion derive

`clifacet::Facet` lets an existing clap command produce validated input without a
second grammar:

```rust
#[derive(clap::Parser, clifacet::Facet)]
#[facet(input = ExampleInput, bridge = build_input)]
pub struct Cli { /* clap attributes unchanged */ }

let input = Cli::try_parse_from(argv)?.validate()?;   // Facet::validate
```

The derive adds no parser. `Facet::command()` *is* `<Cli as CommandFactory>::command()`
— same aliases, same defaults, same value parsers — and `Facet::validate` is the
only route from the parsed struct to `Input`. There are exactly two ways to build
that input, and the derive implements one of them:

| Attribute | Effect |
| --- | --- |
| `#[facet(input = Type)]` | required; the type `validate` returns |
| — (no `bridge`) | copies fields by name, so a renamed or missing field is a compile error |
| `#[facet(bridge = path)]` | calls `path(cli) -> Result<Type, Failure>` instead, for input that normalizes |

The example uses the bridge, because `--name` normalizes from `Option<String>` to a
trimmed `String` and a blank value is rejected; both rules live in
`TryFrom<Cli> for ExampleInput`, next to the data they constrain.

### What the slice proves

| Case | Where |
| --- | --- |
| an alias (`--nickname`, `-n`) resolves to the same typed input as `--name` | `tests/facet_bridge.rs`, `tests/features/facet_bridge.feature` |
| the generated command is clap's own, help and aliases included | `tests/facet_bridge.rs` |
| a clap `default_value_t` survives the bridge | `tests/facet_bridge.rs`, `tests/example_command.rs` |
| a custom `value_parser` (`--level`) rejects unknown values while parsing, in its own words | `tests/facet_bridge.rs` |
| a validation rule the parser cannot express still rejects (`--repeat 9`) | `tests/facet_bridge.rs` |
| the bridge carries every declared field | `tests/facet_bridge.rs` |
| valid and invalid derives, with the diagnostic text as a golden file | `tests/compile_fail.rs`, `tests/ui/` |

Unsupported shapes fail while compiling rather than at runtime, and each has a
`trybuild` case: a derive without `#[facet(input = …)]`, an unknown attribute key,
an enum, and an input missing a field the mirror would copy.

The private-API check is structural: `clifacet` enables no `clap` feature that
exposes internals (`unstable-*`), and both the derive and the trait mention only
`Command`, `CommandFactory`, `Arg` and `ArgMatches` — the documented surface.

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
- **Canonical argv replay.** The derive proves the command that was parsed; it
  does not rebuild argv or re-parse it, so constructs `clap` cannot re-validate
  stay out of scope.
- **Secret handling.** No input is marked secret, so nothing redacts. CLI-14 owns
  the contract, including what may appear in previews and logs.
