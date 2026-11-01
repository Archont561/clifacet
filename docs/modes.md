# Modes and the interaction policy

A run has exactly one mode, decided before anything happens, by a pure function of
three inputs (decision-3):

1. **what the user asked for** — `--interactive`, `--tui`, or nothing;
2. **what the application allows and prefers** — its `Policy`;
3. **what the environment can offer** — its `Capabilities`.

```rust
use clifacet::policy::{Capabilities, Mode, Policy, Request};

let plan = Policy::up_to(Mode::Interactive)
    .resolve(Request::mode(Mode::Interactive), Capabilities::terminal())
    .expect("a terminal can host prompts");

assert_eq!(plan.mode(), Mode::Interactive);

// A request the application does not allow is refused, not silently downgraded.
let refused = Policy::up_to(Mode::Interactive)
    .resolve(Request::mode(Mode::Tui), Capabilities::terminal_with_tui());
assert!(refused.is_err());
```

Two properties make this more than a preference list:

- **A policy is a ceiling, not a wish.** `Policy::up_to(Mode::Interactive)` means
  prompts are allowed and a TUI is not. Interaction can only ever *decrease*: a
  terminal on its own is never consent to prompt.
- **Permission and intent are two answers, not one.** The ceiling says what a run
  *may* do; the preference says what it does when the user asked for nothing. See
  [permission and intent](#permission-and-intent).
- **A refusal has no plan, so it has no frontend.** `Plan` is the only value that can
  build one, through `Plan::with_frontend`, which answers `None` when the mode asks
  nothing. "A forbidden mode never constructs a prompter" is therefore structural
  rather than a convention.

## Permission and intent

`Policy` answers two different questions, and a policy that can only answer the first
has to conflate them: the mode it permits the most becomes the mode it uses by
default. `Policy::prefer` separates them.

```rust
use clifacet::policy::{Capabilities, Mode, Policy, Request};

// Permission: prompts are allowed, a full-screen form is allowed. Intent: prompts are
// what an unadorned run gets.
let policy = Policy::up_to(Mode::Tui).prefer(Mode::Interactive);

let plan = policy
    .resolve(Request::none(), Capabilities::terminal_with_tui())
    .expect("prompts are permitted and possible");
assert_eq!(plan.mode(), Mode::Interactive, "the preference, not the ceiling");

// A request still wins, and is still bounded by the ceiling rather than the preference.
let plan = policy
    .resolve(Request::mode(Mode::Tui), Capabilities::terminal_with_tui())
    .expect("a TUI is permitted here");
assert_eq!(plan.mode(), Mode::Tui);

// Intent can also be *less* interactive than permission: a command that may prompt
// for someone who asks for it does not have to offer a prompt unasked.
let cautious = Policy::up_to(Mode::Tui).prefer(Mode::NonInteractive);
let plan = cautious
    .resolve(Request::none(), Capabilities::terminal())
    .expect("a run that asks nothing always resolves");
assert_eq!(plan.mode(), Mode::NonInteractive);
```

Three rules keep the pair honest:

| Rule | Why |
| --- | --- |
| a preference above the ceiling does not compile | `prefer` checks the ceiling in a `const fn`, so `const POLICY: Policy = Policy::up_to(Mode::Interactive).prefer(Mode::Tui);` is an evaluation error rather than a silent downgrade |
| no stated preference means the ceiling | a policy written before `prefer` existed keeps its behavior, which is why the two are separate values instead of a reinterpreted ceiling |
| the environment still only subtracts | capability steps *down* from the preference, never up, so a terminal can never add interest the application did not state |

Which default an application should choose is a judgement, not a rule: prompts are the
sensible default for a command a person runs by hand, and `Policy::non_interactive()`
or `.prefer(Mode::NonInteractive)` for one whose job is destructive — a prompt that
appears because a terminal happened to be attached is not consent.

## Why "all arguments are supplied" is not a mode trigger

It is tempting to make completeness choose the mode: prompt only when a flag is
missing. The value level already does that, and the mode is the wrong place for it:

- **Defaults make the test vacuous.** A command whose arguments all have defaults is
  "fully specified" on every invocation, so interaction would never happen; the real
  question is whether an argument came from the command line, which is a fact about
  one value rather than about the run.
- **Gap-filling is already the guarantee.** `Session::value(Some(value), …)` returns
  the value untouched and never builds a frontend, so a supplied flag is never
  re-asked — the same prompt run and flag run are one command.
- **Confirmations are not gaps.** Asking "are you sure?" about `--explode` is a
  question about a fully specified invocation. Completeness cannot decide it, so a
  mode chosen by completeness would have to special-case it.
- **A resolvable mode is testable.** `resolve(request, capabilities)` is a pure
  function of three values; a mode that depends on `argv` makes every test case need
  an argument vector to go with it.

The application still decides whether to ask a confirmation at all — the example asks
only when the plan permits it, and `--yes` answers instead of asking.

## The four modes

| Mode | What the user sees | Where the code is |
| --- | --- | --- |
| `NonInteractive` | the flags they typed, and the data | the command and the application |
| `Interactive` | a question only for what the flags left out | `clifacet::prompt` |
| `Tui` | a full-screen form generated from the command | `clifacet::tui` (feature `tui`) |
| none at all | nothing — a refusal before anything runs | `clifacet::policy::Refusal` |

A prompted run and a flag run are **the same command**: the frontend fills only the
gaps a flag left empty, and a supplied value is never re-asked. That is why the
answer does not depend on which mode collected it, and why
[the parity harness](command-core.md#cross-mode-parity-as-a-published-api) can assert
it.

## Streams: data and diagnostics

`stdout` carries only what the caller asked for. Everything a person sees — a
question, a screen, a diagnostic — goes to the diagnostic stream, in every mode:

| Mode | `stdout` | `stderr` |
| --- | --- | --- |
| flags | the requested data | the failure, if any |
| a prompt | the requested data | the questions, then the failure, if any |
| the form | the requested data | the screen, then the failure, if any |
| help, `--version` | the text that was asked for | — |
| a usage error | — | `clap`'s message |

## Exit codes

The numbers belong to the application, not the library. `clifacet::contract::ExitPolicy`
provides the conventional mapping and the categories come from the library:

```rust
use clifacet::contract::ExitPolicy;
use clifacet::{Failure, FailureKind};

let policy = ExitPolicy::standard();

assert_eq!(policy.for_kind(FailureKind::Validation), policy.validation());
assert_eq!(policy.for_failure(&Failure::cancelled("changed their mind")), policy.cancelled());
assert_eq!(policy.success(), 0);
assert_eq!(policy.cancelled(), 130);
```

| Outcome | `ExitPolicy::standard()` | Why |
| --- | --- | --- |
| success | 0 | conventional |
| validation failure | 2 | nothing ran; `clap` documents 2 for the same reason |
| usage error | 2 | the same sentence as validation, so the same code |
| cancellation | 130 | 128 + `SIGINT`; a decision, not a mistake in the invocation |
| execution failure | 1 | it ran and could not finish |

The four *outcomes* a caller distinguishes — success, validation, cancellation and
execution — are pairwise distinct. Usage deliberately shares validation's code, which
the test suite asserts so the overlap reads as a decision.

## Cancellation is its own category

`FailureKind::Cancelled` exists so a frontend can say "the user changed their mind"
without pretending the input was invalid. Answering a confirmation with *no* is a
decision (`Ok(false)`); dismissing the question, or the end of input, is a
cancellation — and only the second costs exit 130.

What a mode may not do is guess: a cancelled form runs nothing, and a refused mode
never reaches the command at all. See [Command core](command-core.md) for the
per-mode detail and the tests behind each row.
