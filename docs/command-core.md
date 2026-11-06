# Command core and its seams

This is the M1 slice: `clap` parses, the application validates, a UI-free command
runs, and the application decides which stream carries the data and what a
failure costs. `examples/simple/src/lib.rs` is the whole thing in one
place — the reference application, in a crate of its own — and
`examples/simple/tests/example_command.rs` drives it through buffers
rather than a terminal. Nothing in it reaches a private path: the crate depends
on `clifacet` exactly as a caller does, so every seam it uses is a published
one.

## The four layers

| Layer | Lives in | Owns | Must not |
| --- | --- | --- | --- |
| `clap` surface | `clifacet_example::Cli` | flags, help, usage errors | know about the command |
| Validation | `TryFrom<Cli> for ExampleInput`, generated from the field declarations and ending in `check_input`, reached through `Facet::validate` | rejecting bad values, defaults | run anything |
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

Measured on `examples/simple` at this commit:

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
| `example --interactive --explode`, answer `n` | 130 | empty | `example: the user declined "Fail on purpose?" (--explode)` |
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

### The example's policy

The reference application enables every frontend the library has — it depends on
`clifacet` with the `tui` feature, and prompts come from the library's own
`Terminal` — so `clifacet_example::POLICY` permits everything:
`Policy::up_to(Mode::Tui).prefer(Mode::Interactive)`. A run therefore never refuses
a mode it was asked for, and an invocation with no flags asks for what the flags left
out instead of taking over the terminal: prompts are what the application *wants*,
the form is what `--tui` *asks* for.

An application that enables fewer frontends sets a lower ceiling: with
`Policy::up_to(Mode::Interactive)`, `--tui` becomes a refusal with exit `2` and a
message naming what was asked for, because a mode nothing can draw is not a mode.
The preference moves the default without moving that boundary — a command whose job
is destructive states `.prefer(Mode::NonInteractive)` and prompts only for a user who
asks it to. See [modes § Permission and intent](modes.md#permission-and-intent). `Capabilities::detected` applies the same
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
// `collect` is generated from the fields' `ask`/`confirm`; it never re-asks.
let output = session.run(&Example, |session| cli.collect(session).and_then(Facet::validate))?;
```

The reference application writes no prompt code: `#[clifacet(.., ask = "Who should I
greet?")]` on `--name` and `confirm = "Fail on purpose?"` on `--explode` generate
`Collect::collect`, which calls `Session::ask_for` and `Session::confirm_flag` — the
same session rules below, applied per declared field.

- **A supplied value is never re-asked.** Parity is a property of `Session::value`,
  not a promise: the same `--name ada` and the prompted `ada` reach the same input
  through the same validation, which a BDD scenario asserts by comparing the data.
- **A missing value is asked for only when the plan permits.** `Session::start`
  builds the frontend through `Plan::with_frontend`, so a non-interactive run has
  no frontend at all and a question is never asked, let alone answered.
- **`--yes` answers confirmations and nothing else.** `Session::confirm` settles a
  confirmation as `Confirmation::Assumed` without asking, and one with nothing able to
  ask as `Confirmation::Unavailable`. `is_confirmed()` is the fail-safe boolean: only
  `Confirmed` and `Assumed` proceed, so a caller that does not care *how* still fails
  safe, and one that does — "pass `--yes`" is a better message than "declined" —
  matches on the outcome instead of inspecting the plan.
- **Collection precedes execution.** `Session::run` collects and then executes, so
  a cancelled question, a bad answer or an exhausted script returns before the
  command is reachable.

Three adapters ship, all of them implementations of the same `Prompter` seam — two
required methods, `ask` and `confirm`, and provided ones a frontend may override:

| Adapter | Feature | What it is |
| --- | --- | --- |
| `Terminal` | none | a question written to the *diagnostic* stream and a line read from `stdin`; end of input is a cancellation rather than an empty answer; a choice is a numbered list that takes a number or a name and re-asks on anything else |
| `Dialog` | `interactive` | the same questions as an `inquire` dialog — a cursor, a help line, bracketed paste — drawn on `stderr`, with a choice as a list to move through |
| `Scripted` | none | answers from a list, a choice by its exact label; the seam every test uses, so a flow is driven end to end without a terminal and cannot reach one by accident |

`Dialog` is the one a real process usually wants, and it changes nothing else: the
session still decides whether a question may be asked, a supplied value is still
never re-asked, and `--yes` still answers confirmations without a dialog appearing.
Escape or Ctrl-C is a `FailureKind::Cancelled`; a dialog that cannot reach a terminal
is an *execution* failure that says so, rather than a question nobody can answer. That
is checked on stdin before `inquire` is entered, because how a raw-mode switch fails is
the platform's business — one `isatty` makes "no terminal" a reported failure
everywhere instead of a wait on some platforms.

Because presentation is a seam and not a setting, an application can ship its own.
Implement `Prompter` — `ask` and `confirm` are the two it requires; `choose` is provided
as a numbered list read through `ask`, so a frontend written before choices existed
keeps compiling — and hand it to `Session::start`:

```rust
use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Dialog, Prompter, Session};

// Presentation is the application's choice: the library's dialog, or any `Prompter`
// of its own. Constructing one touches no terminal.
let build_dialog = || Box::new(Dialog::new()) as Box<dyn Prompter>;

// A run that may ask nothing never calls that builder, which is what keeps a piped
// run non-interactive however rich the frontend is.
let plan = Policy::non_interactive()
    .resolve(Request::none(), Capabilities::terminal())
    .expect("the run resolves");
let mut session = Session::start(plan, build_dialog);

assert_eq!(
    session.value(None, "Which license?").expect("nothing may ask"),
    None,
    "a missing value stays missing, and the dialog is never built"
);
```

On a plan that *does* permit questions, the same builder is called once and the dialog
asks what the flags left out.

What a custom frontend cannot change is the decision that allowed it: the builder is
only called when the plan asks something, so a richer widget can never turn a piped
run into an interactive one. `examples/simple/src/lib.rs` is the worked example — the
process uses `dialog_frontend()` and keeps `terminal_frontend()` beside it to show the
choice is the application's.

`FailureKind::Cancelled` is how "the user changed their mind" travels. Answering a
confirmation with `no` is a *decision* (`Confirmation::Declined`); dismissing the
question, or end of input, is a *cancellation*. Only the second costs exit `130`.

### Questions beyond text and yes/no

`Session::choose` asks the user to pick one of a fixed set of values. Each value is
shown by its `Display` text, and the value itself is what comes back — never its label
read back and parsed — so the set of choices and the type they produce cannot drift. It
keeps the session's rule: a supplied value is returned untouched, even one that is not
among the choices, because checking it is the parser's job and prompting never changes
an explicit choice.

```rust
use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Scripted, Session};

let plan = Policy::up_to(Mode::Interactive)
    .resolve(Request::none(), Capabilities::terminal())
    .expect("prompts are permitted here");
let mut session = Session::start(plan, || {
    // A script picks by label, exactly as offered.
    Box::new(Scripted::new([Answer::Text(String::from("in progress"))]))
});

let statuses = ["to do", "in progress", "done"];
assert_eq!(
    session.choose(None, "Which status?", statuses).expect("one was picked"),
    Some("in progress"),
);
assert_eq!(
    session.choose(Some("done"), "Which status?", statuses).expect("never asked"),
    Some("done"),
);
```

A frontend sees only `Choice` labels, with optional help, and answers with a position;
the session maps it back, and a position it never offered is the frontend's failure.

`Session::ask_secret` asks for a credential without echoing it. The answer is a
`Secret` from the moment it arrives, and the session's provenance records *that* it was
prompted for, flagged secret — never what it was:

```rust
use clifacet::policy::{Capabilities, Mode, Policy, Request};
use clifacet::prompt::{Answer, Scripted, Session};
use clifacet::provenance::Source;

let plan = Policy::up_to(Mode::Interactive)
    .resolve(Request::none(), Capabilities::terminal())
    .expect("prompts are permitted here");
let mut session = Session::start(plan, || {
    Box::new(Scripted::new([Answer::Text(String::from("s3cret"))]))
});

let token = session
    .ask_secret(None, "token", "API token?")
    .expect("answered")
    .expect("asked");
assert_eq!(token.expose(), "s3cret");
assert_eq!(token.to_string(), "[redacted]");
assert_eq!(session.provenance().source_of("token"), Some(Source::Prompt));
assert!(session.provenance().is_secret("token"));
```

Hiding input is the frontend's job, and a frontend that cannot do it refuses: the
provided `Prompter::ask_secret` fails rather than fall back to `ask`, so neither
`Terminal` nor a frontend written before secrets existed can echo one by accident.
Every answered `ask_for` is recorded the same way, unflagged, under its argument's id.

`Session::collect_values` asks one question repeatedly to build a list, and an empty
or blank answer ends it — the session does not reword the question, so the question
should say so. Stopping part-way is not finishing: a dismissal, or a frontend that
breaks, after two of three values returns the failure and discards the two, so a
partially collected list is never handed back as the list the user meant.

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

Five rules keep it a frontend rather than a second command:

- **A form is generated from the command, and asked for explicitly.**
  `FormState::of::<Cli>()` generates it from the fields declared `form` — that is
  `FormState::generated(&Cli::command(), Cli::form_fields())` — and masks the fields
  declared `secret`, so a credential is hidden because it was declared one rather than
  because a second call was remembered. Labels come from the arguments' help and the
  initial values from their defaults. Which arguments a
  screen asks for is a design decision, so it is declared, never guessed: the fields
  marked `form` (`name`, `repeat`, `level`), not the flags that chose the mode, which
  are not questions for someone who already chose one. A name the command does not
  have is an error, not an empty form, and each answer is submitted under the flag
  clap knows it by.
- **A field is what its grammar says.** A `SetTrue`/`SetFalse` flag is a toggle, drawn
  `[x]`/`[ ]`, that Space flips and that submits the bare flag only when it differs
  from the default — `--dry-run yes` is not something clap accepts. An argument with
  possible values is a choice, drawn `‹green›`: Space moves to the next option, a
  letter jumps to the next option starting with it, and an optional choice without a
  default passes through `‹(none)›`, which submits nothing. Everything else is text.
  A list — a repeatable or counted flag, or more than one value — is refused by
  `FormState::generated`, because cutting it to one value would be a silent lie;
  `FieldState::choices()`, `is_toggle()` and `is_on()` let a custom field draw the
  same kinds its own way.
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
entering and leaving it. `examples/simple/examples/tui_custom_screen.rs` is
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

`examples/simple/examples/parity_report.rs` runs the example command three
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

### Provenance from the parse

Nothing has to be recorded by hand. `Provenance::from_matches` reads where `clap` found
each value, and `merge` adds what the session asked for, so one record covers a run
whatever mixture of flags, environment, defaults and prompts produced it:

```rust
use clap::{CommandFactory, FromArgMatches, Parser};
use clifacet::provenance::{Provenance, Source};

#[derive(Debug, Parser)]
#[command(name = "deploy")]
struct Deploy {
    #[arg(long)]
    name: Option<String>,
    #[arg(long, default_value_t = 1)]
    repeat: u8,
    #[arg(long)]
    token: Option<String>,
}

let command = Deploy::command();
let matches = command
    .clone()
    .try_get_matches_from(["deploy", "--token", "s3cret"])
    .expect("a valid invocation");
let parsed = Deploy::from_arg_matches(&matches).expect("the matches are this command's");

let mut provenance = Provenance::from_matches(&command, &matches).with_secrets(&["token"]);
let mut asked = Provenance::default();
asked.record("name", Source::Prompt); // what `session.provenance()` holds after asking
provenance.merge(&asked);

assert_eq!(parsed.repeat, 1);
assert_eq!(provenance.source_of("repeat"), Some(Source::Default));
assert_eq!(provenance.source_of("name"), Some(Source::Prompt));
assert!(!provenance.to_string().contains("s3cret"));
```

- **Three sources come from `clap`.** A typed value is `argv`, one read from an
  argument's `env` is `environment`, and one `clap` filled in — a `default_value`, or
  `false` for a flag that was not given — is `default`.
- **An absent value records nothing.** An `Option` nobody supplied has no source, and
  inventing one is the lie this exists to avoid; if the session then asks for it, the
  prompt is its only record.
- **The command is passed with the matches** because `ArgMatches` lists the derive's
  argument group beside the arguments and cannot tell them apart. The chosen subcommand
  is walked too.
- **Secrets are named, not detected.** The parse does not know which values are
  credentials; `with_secrets` marks them, and a command declared with
  `#[clifacet::command]` lists its own in `Facet::secret_fields()`.

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
- **A line terminal cannot hide a secret, so it does not ask for one.**
  `prompt::Terminal` reads a line from stdin and has no way to switch echo off;
  `Session::ask_secret` through it fails with an execution error rather than show what
  is typed and leave it in scrollback. `Dialog` (feature `interactive`) asks blind.
- **Plain values are not protected.** A credential held as a `String` prints as usual;
  the guarantee starts at `Secret<T>`.
- **Errors from outside the framework** — a `clap` value parser's message, an I/O
  error, a third-party crate's diagnostic — are formatted by their authors.

## The companion derive

`clifacet::Facet` lets an existing clap command produce validated input without a
second grammar:

```rust,ignore
#[clifacet::command(input = ExampleInput, check = check_input)]
pub struct Cli { /* one #[clifacet(..)] per field */ }

let input = Cli::try_parse_from(argv)?.validate()?;   // Facet::validate
```

The derive adds no parser. `Facet::command()` *is* `<Cli as CommandFactory>::command()`
— same aliases, same defaults, same value parsers — and `Facet::validate` is the
only route from the parsed struct to `Input`. There are exactly two ways to build
that input, and the derive implements one of them:

| Attribute | Effect |
| --- | --- |
| `#[facet(input = Type)]` | required; the type `validate` returns: a path, a generic type, an alias or a projection — [what `input` may name](derive.md#what-input-may-name) |
| — (no `bridge`) | copies fields by name, so a renamed or missing field is a compile error, applying each field's declared rules on the way |
| `#[facet(check = path)]` | ends the generated bridge with `path(&input) -> Result<(), Failure>` |
| `#[facet(bridge = path)]` | calls `path(cli) -> Result<Type, Failure>` instead, for input the declarations cannot describe |

The example declares its rules on the fields they constrain: `--name` is filled with
`world`, trimmed and refused when blank, and the credential is trimmed with blank
meaning none, all by `#[clifacet(..)]` keys. The one rule no key states, the
`--repeat` range, is `check_input`, which runs after them on the built input. So
`--name "  " --repeat 9` reports the blank name: a field's own rule comes first. The
key table and the compile errors are in [the derive guide](derive.md).

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
- **`--json`.** The library writes no JSON: output is the application's (decision-3),
  so the flag and the stream are too. With the `serde` feature a `Failure` serialises
  as `{"kind":…,"message":…}` and a `Secret` as `[redacted]`, which is what an
  application builds a JSON failure body from with `serde_json`.
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
- **Secret-aware prompting through the derive.** `Session::ask_secret` asks without
  echo and records the prompt in the session's provenance, but the derive still refuses
  `ask` on a `secret` field, and where a *supplied* value came from — argv or the
  environment — is recorded by the application rather than inferred. Redaction itself
  is covered; see "What this cannot do" above for the boundaries.
