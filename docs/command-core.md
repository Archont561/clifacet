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
| `example --tui` (built with `tui`) | 0 | the form, then `hello, world` | empty |
| `example --tui` (built without `tui`) | 2 | empty | `example: a TUI is not available: this command allows at most interactive use` |
| `example --interactive` (a terminal) | 0 | `hello, ada` after answering `ada` | empty |
| `example --interactive --explode`, answer `n` | 130 | empty | `example: the user declined to fail on purpose (--explode)` |
| `example --yes` | 0 | `hello, world` | empty |
| `example --interactive --tui` | 2 | empty | `clap` usage error (the two conflict) |
| closed data stream | 1 | — | `example: cannot write the output: …` |

Two details worth keeping:

- **A range check is not a parser's job.** `--repeat 255` parses as a `u8`, so the
  bound lives in validation and is tested there.
- **Help and `--version` are success on the data stream**, because they are the
  result the caller asked for. `clap` decides that with `Error::use_stderr`, and
  the application follows it instead of guessing.

## Interaction policy

Which mode a run uses is decided before anything happens, and the decision is a
pure function of three inputs (decision-3): the **explicit request**, the
**application's policy**, then what the **environment can offer**.

```rust
let plan = Policy::up_to(Mode::Interactive)
    .resolve(Request::mode(Mode::Interactive), Capabilities::terminal())?;
match plan.mode() { Mode::Interactive => …, Mode::NonInteractive => … }
```

Two properties make this more than a preference list:

- **A policy is a ceiling, not a wish.** `Policy::up_to(Mode::Interactive)` means
  prompts are allowed and a TUI is not; without an explicit request the most
  interactive permitted mode the environment supports is chosen, so interaction
  can only ever *decrease*. A terminal on its own is never consent to prompt.
- **A refusal has no plan, so it has no frontend.** `Plan` is the only value that
  can build one, through `Plan::with_frontend`, which answers `None` when the mode
  asks nothing. There is no second door, which is why "forbidden modes do not
  construct a prompter" is a testable claim rather than a promise.

`--yes` answers confirmations and decides nothing else: it does not change the
mode, does not imply a mode, and does not affect the exit code.

### The example's ceiling

`example::POLICY` allows prompts, and allows a TUI only when the crate is built
with the `tui` feature. Without it `--tui` is refused with exit `2` and a message
naming what was asked for: a mode nothing can draw is not a mode, so promising it
would be a lie. `Capabilities::detected` applies the same rule from the other side
— the TUI capability is the feature *and* a terminal — which is why the policy can
never resolve to a mode the binary cannot render.

### Cancellation is its own category

`FailureKind::Cancelled` exists so a frontend can report "the user changed their
mind" without pretending the input was invalid. The example maps it to exit `130`
— conventional for interruption, and deliberately not `2`. A cancelled run and a
refused mode both execute nothing, but only one of them is a mistake in the
invocation.

## Prompting, and why it is not a second flow

An interactive run and a flag run are the same command. The frontend only fills the
gaps a flag left empty, and that rule lives in one place:

```rust
let mut session = Session::start(plan, || Box::new(Terminal::stdio()));
let name = session.value(cli.name.take(), "Who should I greet?")?;   // never re-asks
let output = session.run(&Example, |session| collect(session, cli))?; // collect, then run
```

- **A supplied value is never re-asked.** Parity is a property of `Session::value`,
  not a promise: the same `--name ada` and the prompted `ada` reach the same input
  through the same validation, which a BDD scenario asserts by comparing the data.
- **A missing value is asked for only when the plan permits.** `Session::start`
  builds the frontend through `Plan::with_frontend`, so a non-interactive run has
  no frontend at all and a question is never asked, let alone answered.
- **`--yes` answers confirmations and nothing else.** `Session::confirm` returns
  `true` without asking; a confirmation with nothing able to ask returns `false`,
  so a caller that forgets to check the plan fails safe instead of proceeding.
- **Collection precedes execution.** `Session::run` collects and then executes, so
  a cancelled question, a bad answer or an exhausted script returns before the
  command is reachable.

Two adapters ship. `Terminal` asks a person: questions go to the *diagnostic*
stream so the data stream stays machine-readable, end of input is a cancellation
rather than an empty answer, and a typo re-asks instead of guessing. `Scripted`
answers from a list and is the seam every test uses — a flow can be driven end to
end without a terminal and cannot reach one by accident.

`FailureKind::Cancelled` is how "the user changed their mind" travels. Answering a
confirmation with `no` is a *decision* (`Ok(false)`); dismissing the question, or
end of input, is a *cancellation*. Only the second costs exit `130`.

## The full-screen form

The `tui` feature adds a full-screen frontend, and the components it draws with are
public: an application that wants a different screen builds its own out of the same
pieces (decision-4).

| Item | What it is |
| --- | --- |
| `tui::FormState`, `tui::FieldState` | the form: values, focus, the last refusal. No `ratatui` in sight |
| `tui::Key`, `tui::Outcome` | the keystrokes a form answers, and what it decided about one |
| `tui::Form`, `tui::Field` | `ratatui` widgets — and the generated form is nothing but fields |
| `tui::app::run` | the loop: draw, read, submit |
| `tui::ratatui` | the frontend library at the version this crate was built against |

Four rules keep it a frontend rather than a second command:

- **A form is generated from the command, and asked for explicitly.**
  `FormState::generated(&Cli::command(), &["name", "repeat", "level"])` takes the
  labels from the arguments' help and the initial values from their defaults. Which
  arguments a screen asks for is a design decision, so it is named, never guessed:
  the flags that chose the mode are not questions for someone who already chose one.
  A name the command does not have is an error, not an empty form.
- **Submission goes through `clap`.** The answers become arguments,
  `Cli::try_parse_from` parses them, and `Facet::validate` plus the handler run as
  they would for flags. A rejected answer — `--repeat 9`, `--repeat x` — keeps the
  form open and shows the same words the flags would print, so a typo costs an edit
  instead of the whole run. The BDD scenario asserts the data a typed `ada` produces
  is byte-for-byte what `--name ada` produces.
- **A prefilled value is an offer.** Typing replaces a default the user has not
  edited; backspace edits it like any other text. Appending to a default would make
  "change 1 to 3" mean "delete the 1 first".
- **The state machine is screen-free.** `Key` is a key, not a crossterm event, so
  focus, editing, submission and cancellation are asserted against a `TestBackend`
  and a scripted key list. The terminal appears in exactly two places: `live_keys`
  reads the process's event stream, and `app::run_on` enters and leaves the
  alternate screen — and leaves it on every path, cancellation included.

### The two escape hatches

decision-4 asks for composition without a second command, and ships exactly two ways
to replace something, both explicit:

```rust
// 1. One field, drawn by the application.
Form::new(&state).with_field("name", &Masked);

// 2. The whole screen, drawn by the application.
app::run_with(&mut terminal, &mut state, keys, my_screen, example::tui_submit);
```

Neither is a registry and neither can change behaviour. `CustomField::draw` is given
the field, its value and whether it has the focus, because a widget value built once
would show a stale answer; `run_with` replaces the drawing and nothing else — the
keys, the refusals, the submission and the handler are the same loop, so a custom
screen reaches the same validation as the generated form. `run_on_with` is the same
hatch with the alternate screen, so a custom screen is not left to reimplement
entering and leaving it. `examples/tui_custom_screen.rs` is both hatches in one
runnable screen.

The one thing a headless test cannot reach is the process's own terminal: the size
probe, the raw-mode restore and the live event stream. Everything they wrap is
covered, which is why `cargo llvm-cov` reports 815 of 820 lines rather than all of
them.

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

- **Widgets beyond the form.** The TUI ships a labelled field and a form. Tables,
  scroll views, the whole-screen escape hatches (CLI-12) and theming are not there,
  and neither is a universal widget-backend abstraction — tests use `ratatui`'s own
  `TestBackend`, which is why `tui::ratatui` is re-exported.
- **`--json`.** The `json` feature reserves `serde_json`; no flag selects it, so
  only the human data line exists.
- **Mode detection heuristics.** `Capabilities::detected` reports a terminal and
  nothing else; no `TERM`, locale or window-size probing. A false positive there
  would resolve to a mode that cannot render.
- **Cancellation from a real frontend.** The category, the message and the exit
  code exist and are tested against `Scripted` and against the TUI's scripted keys;
  an actual person pressing <kbd>Esc</kbd> is the one path no test can take.
- **Canonical argv replay.** The derive proves the command that was parsed; it
  does not rebuild argv or re-parse it, so constructs `clap` cannot re-validate
  stay out of scope.
- **Secret handling.** No input is marked secret, so nothing redacts. CLI-14 owns
  the contract, including what may appear in previews and logs.
