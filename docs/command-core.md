# Command core and its seams

This is the M1 slice: `clap` parses, the application validates, a UI-free command
runs, and the application decides which stream carries the data and what a
failure costs. `crates/clifacet-example/src/lib.rs` is the whole thing in one
place — the reference application, in a crate of its own — and
`crates/clifacet-example/tests/example_command.rs` drives it through buffers
rather than a terminal. Nothing in it reaches a private path: the crate depends
on `clifacet` exactly as a caller does, so every seam it uses is a published
one.

## The four layers

| Layer | Lives in | Owns | Must not |
| --- | --- | --- | --- |
| `clap` surface | `clifacet_example::Cli` | flags, help, usage errors | know about the command |
| Validation | `TryFrom<Cli> for ExampleInput`, reached through `Facet::validate` | rejecting bad values, defaults | run anything |
| Command | `impl Command for Example` | the operation | touch a stream, read a terminal, know `clap` exists |
| Application | `clifacet_example::run`, `clifacet_example::main` | streams, exit codes, diagnostics | contain the operation |

`Command` is the only seam between the middle two rows, and it is the only one
that has to stay stable for later frontends:

```rust,ignore
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

**Streams are the application's.** `Command` never writes.
`clifacet_example::run` takes `out` and `err` as parameters, which is what makes a
run testable without a terminal and is also why "stdout carries only requested
machine data" is a property a test can assert rather than a convention.

## Demonstrated behavior

Measured on `crates/clifacet-example` at this commit:

| Invocation | Exit | stdout | stderr |
| --- | --- | --- | --- |
| `example` | 0 | `hello, world` | empty |
| `example --repeat 3` | 0 | three lines | empty |
| `example --repeat 0` | 2 | empty | `example: --repeat must be between 1 and 8, got 0` |
| `example --explode` | 1 | empty | `example: the example operation failed on purpose (--explode)` |
| `example --nope` | 2 | empty | `clap` usage error |
| `example --help` | 0 | help text | empty |
| `example --token <revoked…>` | 1 | empty | `example: the credential [redacted] was rejected` |
| `example --tui` | 0 | the form, then `hello, world` | empty |
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

```rust,ignore
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

The reference application enables every frontend the library has — it depends on
`clifacet` with the `tui` feature, and prompts come from the library's own
`Terminal` — so `clifacet_example::POLICY` is `Policy::up_to(Mode::Tui)` and a run
never refuses a mode it was asked for. An application that enables fewer
frontends sets a lower ceiling: with `Policy::up_to(Mode::Interactive)`, `--tui`
becomes a refusal with exit `2` and a message naming what was asked for, because
a mode nothing can draw is not a mode. `Capabilities::detected` applies the same
rule from the other side — the TUI capability is the feature *and* a terminal —
which is why the policy can never resolve to a mode the binary cannot render.

### Cancellation is its own category

`FailureKind::Cancelled` exists so a frontend can report "the user changed their
mind" without pretending the input was invalid. The example maps it to exit `130`
— conventional for interruption, and deliberately not `2`. A cancelled run and a
refused mode both execute nothing, but only one of them is a mistake in the
invocation.

## Prompting, and why it is not a second flow

An interactive run and a flag run are the same command. The frontend only fills the
gaps a flag left empty, and that rule lives in one place:

```rust,ignore
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

```rust,ignore
// 1. One field, drawn by the application.
Form::new(&state).with_field("name", &Masked);

// 2. The whole screen, drawn by the application.
app::run_with(&mut terminal, &mut state, keys, my_screen, clifacet_example::tui_submit);
```

Neither is a registry and neither can change behaviour. `CustomField::draw` is given
the field, its value and whether it has the focus, because a widget value built once
would show a stale answer; `run_with` replaces the drawing and nothing else — the
keys, the refusals, the submission and the handler are the same loop, so a custom
screen reaches the same validation as the generated form. `run_on_with` is the same
hatch with the alternate screen, so a custom screen is not left to reimplement
entering and leaving it. `crates/clifacet-example/examples/tui_custom_screen.rs` is
both hatches in one runnable screen.

The one thing a headless test cannot reach is the process's own terminal: the size
probe, the raw-mode restore and the live event stream. Everything they wrap is
covered, which is why `cargo llvm-cov` reports 932 of 937 lines rather than all of
them.

## Cross-mode parity, as a published API

A flag run, a prompted run and a full-screen form are the same command, and
`clifacet::harness` is how an application proves it instead of promising it:

```rust,ignore
let (command, effects) = Recording::new(MyCommand, describe);
let flags = Outcome::measured(|out, err| app::main_with_command(&command, argv, headless(), .., out, err))
    .with_effects(effects.recorded());

Parity::new("flags", flags)
    .compare("scripted prompt", prompted)
    .compare("headless tui", typed)
    .assert_parity();
```

Three pieces, and deliberately no fourth:

| Piece | What it does |
| --- | --- |
| `Outcome` | one run reduced to the four comparable subjects: exit code, data, diagnostic, effects |
| `Recording` | wraps a `Command` and records the input each execution received, failures included |
| `Parity` | compares runs, and names the mode and the subject that disagreed |

- **The harness runs nothing itself.** Each mode is the application's own entry
  point, so a comparison covers the real path — `clap` parsing, the policy, the
  prompt session, the form — rather than a test-only runner that could drift. There
  is no mode enum and no runner trait, because describing frontends to the library is
  the global registration decision-4 forbids.
- **Recording is "what reached the command", not "what was typed".** A rejected
  answer leaves the record empty, which is how "nothing ran" becomes an assertion
  instead of an inference from an empty data stream.
- **Subjects are selectable.** Frontends word a cancellation differently on purpose —
  one names the question, the other names the form — so `Parity::comparing` names the
  subjects a scenario is actually about. Everything else compares all four.

`crates/clifacet-example/examples/parity_report.rs` runs the example command three
ways and prints whether the modes agree; it lives outside the library crate, so it
can only use this published surface, which is the point.

| Case | Where |
| --- | --- |
| three modes reach the command with the same validated input | `clifacet-example/tests/harness_parity.rs`, `clifacet-example/tests/features/parity.feature` |
| a successful run is identical in every mode | `clifacet-example/tests/harness_parity.rs` |
| a validation failure keeps the flags' words, and runs nothing in any mode | `clifacet-example/tests/harness_parity.rs` |
| dismissing a question and leaving the form are the same outcome | `clifacet-example/tests/harness_parity.rs` |
| an execution failure costs the same exit code | `clifacet-example/tests/harness_parity.rs` |
| measuring, recording and comparing, without any frontend | `clifacet/tests/harness.rs` |

## Streams and exit codes

Two contracts hold in every mode, and both are asserted rather than assumed.

**The data stream carries only data.** Everything a person sees goes to the diagnostic
stream: a question, a full-screen interface, a diagnostic. That is why a form draws on
`stderr` — drawing it on `stdout` would put escape sequences in the output a caller
asked for — and why a prompt writes its question to `stderr` while the answer's effect
appears on `stdout`.

| Mode | `stdout` | `stderr` |
| --- | --- | --- |
| flags | the requested data | the failure, if any |
| a prompt | the requested data | the questions, then the failure, if any |
| the form | the requested data | the screen, then the failure, if any |
| help, `--version` | the text that was asked for | — |
| a usage error | — | `clap`'s message |

**Each outcome has its own code.** `clifacet::contract::ExitPolicy` holds the mapping,
`ExitPolicy::standard()` is the conventional one, and the reference application names it
`clifacet_example::EXIT` so the constants and the policy cannot drift:

| Outcome | Code | Why |
| --- | --- | --- |
| success | 0 | conventional |
| validation failure | 2 | nothing ran; `clap` documents 2 for the same reason |
| usage error | 2 | the same sentence as validation, so the same code |
| cancellation | 130 | 128 + `SIGINT`; a decision, not a mistake |
| execution failure | 1 | it ran and could not finish |

The four *outcomes* a caller distinguishes — success, validation, cancellation and
execution — are pairwise distinct, which a test asserts; usage deliberately shares
validation's code, which is asserted too so the overlap reads as a decision.

`Capabilities::detected` follows the stream rule: a full-screen interface needs the
`tui` feature, a terminal for the screen (`stderr`) and a terminal for the data
(`stdout`), because a piped data stream is asking for data, not for a screen.

| Case | Where |
| --- | --- |
| the policy maps each category, and the four outcomes are distinct | `clifacet-example/tests/outcomes.rs` |
| a question, a screen and a diagnostic never reach the data stream | `clifacet-example/tests/outcomes.rs`, `clifacet-example/tests/harness_parity.rs` |
| success, validation and a question-answered run, as scenarios | `clifacet-example/tests/features/outcomes.feature` |
| help is data and a usage error is a diagnostic | `clifacet-example/tests/example_command.rs`, `clifacet-example/tests/outcomes.rs` |

## Secrets and provenance

A value wrapped in `clifacet::secret::Secret` is redacted wherever the framework
speaks, and `clifacet::provenance` records where a value came from without recording
the value.

```rust,ignore
#[arg(long, value_name = "TOKEN")]
pub token: Option<Secret<String>>,          // clap parses it, nothing prints it
```

Redaction is a property of the type, not a scrubbing pass:

| Surface | Why it is covered |
| --- | --- |
| a diagnostic the application writes | `format!("{token}")` is `[redacted]` |
| a `Debug` record, such as a parity harness effect | `Secret([redacted])` |
| a preview, such as a form field | a secret field draws `[redacted]`, and its length is not reported |
| a failing test's panic message | the record it prints is the `Debug` above |

- **Nothing keeps a list of secret values.** A registry of live credentials would
  itself be a place to leak them, and it would have to be cleansed. Since the marker
  is produced by the value's own formatting, there is nothing to cleanse.
- **`Secret::expose` is the only read.** No `Deref`, no `AsRef`, no `Serialize`: a
  credential reaches a log, a file or a URL only through a line someone wrote, which
  is what makes review possible.
- **Provenance has no field for a value.** `Origin` holds a name, a `Source` and
  whether the value is secret, so "the record cannot leak a secret" is structural. It
  answers the question a parity test asks after it has already asserted that the answer
  itself was the same in every mode.
- **Masking is presentation.** A secret form field submits its real value; only the
  drawing changes. Masking the characters instead of replacing them would still report
  the length.

### What this cannot do

The framework's promise is about the framework's own output. Everything below is the
application's, and is listed so absence is not mistaken for oversight:

- **Application-owned logs.** If the application calls `expose()` and writes the value
  to a log, a file or a telemetry event, no framework type is involved and nothing here
  can redact it.
- **Command-line arguments are public on most systems.** A secret passed as a flag is
  visible in `ps`, in `/proc/<pid>/cmdline`, to the process supervisor and in shell
  history. Prefer an environment variable or a prompt; the framework cannot fix argv.
- **Environment variables are readable by the same user,** through
  `/proc/<pid>/environ` and through any crash dump or core file. They are safer than
  argv, not safe.
- **A prompted secret is echoed.** `prompt::Terminal` reads a line from stdin, so a
  terminal shows what is typed and keeps it in scrollback. A no-echo prompt is not
  implemented.
- **Plain values are not protected.** A credential held as a `String` prints as usual;
  the guarantee starts at `Secret<T>`.
- **Errors from outside the framework** — a `clap` value parser's message, an I/O
  error, a third-party crate's diagnostic — are formatted by their authors.

## The companion derive

`clifacet::Facet` lets an existing clap command produce validated input without a
second grammar:

```rust,ignore
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
| an alias (`--nickname`, `-n`) resolves to the same typed input as `--name` | `clifacet-example/tests/facet_bridge.rs`, `clifacet-example/tests/features/facet_bridge.feature` |
| the generated command is clap's own, help and aliases included | `clifacet-example/tests/facet_bridge.rs` |
| a clap `default_value_t` survives the bridge | `clifacet-example/tests/facet_bridge.rs`, `clifacet-example/tests/example_command.rs` |
| a custom `value_parser` (`--level`) rejects unknown values while parsing, in its own words | `clifacet-example/tests/facet_bridge.rs` |
| a validation rule the parser cannot express still rejects (`--repeat 9`) | `clifacet-example/tests/facet_bridge.rs` |
| the bridge carries every declared field | `clifacet-example/tests/facet_bridge.rs` |
| valid and invalid derives, with the diagnostic text as a golden file | `clifacet/tests/compile_fail.rs`, `clifacet/tests/ui/` |

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
- **A parity runner.** `clifacet::harness` measures and compares runs; it does not
  provide a mode enum or run an application's frontends for it. Each application
  knows how its own entry points are called.
- **Mode detection heuristics.** `Capabilities::detected` reports a terminal and
  nothing else; no `TERM`, locale or window-size probing. A false positive there
  would resolve to a mode that cannot render.
- **Cancellation from a real frontend.** The category, the message and the exit
  code exist and are tested against `Scripted` and against the TUI's scripted keys;
  an actual person pressing <kbd>Esc</kbd> is the one path no test can take.
- **Canonical argv replay.** The derive proves the command that was parsed; it
  does not rebuild argv or re-parse it, so constructs `clap` cannot re-validate
  stay out of scope.
- **Secret-aware prompting.** A credential cannot be asked for without echo, and
  `Provenance` is recorded by the application rather than inferred. Redaction itself
  is covered; see "What this cannot do" above for the boundaries.
