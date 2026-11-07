# The simple example

One command, end to end: `clap` parses the flags, the input is validated once, a
UI-free command runs, and the application decides which stream gets the data and what
each outcome costs. Everything in it is the library's public API — the crate depends on
`clifacet` the way any consumer does, which is what makes it a proof of that API rather
than a part of it. It is `publish = false`: a demonstration, not a third crate.

## Run it

From the repository root:

```sh
cargo run -p clifacet-example                             # asks for what the flags left out
cargo run -p clifacet-example -- --name ada --repeat 2     # flags only
cargo run -p clifacet-example -- --interactive             # prompts
cargo run -p clifacet-example -- --tui                     # the full-screen form
cargo run -p clifacet-example -- --name ada --explode      # a failure the app owns
```

A piped run asks nothing, so it is safe to script:

```sh
cargo run -p clifacet-example -- --name ada | cat
```

## The command

| Flag | Meaning | Default |
| --- | --- | --- |
| `--name <NAME>` (`-n`, alias `--nickname`) | who to greet | asked for, or `world` |
| `--repeat <COUNT>` | how many times, 1 to 8 | `1` |
| `--level <LEVEL>` | `basic` or `verbose` | `basic` |
| `--explode` | fail on purpose, to show the failure seam | off |
| `--token <TOKEN>` | a credential that is never printed; `revoked…` is refused | none |
| `--interactive` | refuse to fall back to flags: ask, or fail if there is no terminal | off |
| `--tui` | open the full-screen form instead | off |
| `--yes` | answer confirmations with yes | off |

`--interactive` and `--tui` conflict: a run has exactly one mode.

## The four modes

| Mode | What happens | How to get it |
| --- | --- | --- |
| non-interactive | flags in, data out — nothing is ever asked | no terminal, or `--name` and friends with a pipe |
| interactive | an `inquire` dialog asks only for what the flags left out | the default on a terminal, or `--interactive` |
| TUI | a full-screen form generated from the command | `--tui` |
| refused | nothing runs, and the diagnostic says why | ask for a mode this binary cannot draw |

The application states both halves of that policy in one line:

```text
pub const POLICY: Policy = Policy::up_to(Mode::Tui).prefer(Mode::Interactive);
```

The dialog is the application's choice, not the framework's: `src/entry.rs` names
`dialog_frontend()` for the process and keeps `terminal_frontend()` beside it, a plain
question-and-line-reader built on the same two-method `Prompter` seam. Swapping them
changes no other line, because the session decides *whether* a question may be asked
and a supplied value is never re-asked either way.

`up_to` is permission — both frontends are enabled here — and `prefer` is intent:
prompts are what an unadorned run gets, and the form is what `--tui` asks for. A
`Policy::up_to(Mode::Interactive)` application refuses `--tui` with exit `2` instead.
[The modes guide](../../docs/modes.md#permission-and-intent) is the full rule.

## Streams and exit codes

`stdout` carries only the data the caller asked for. Every question, screen and
diagnostic goes to `stderr`, in every mode.

| Outcome | Exit code |
| --- | --- |
| the greeting was written | `0` |
| help or `--version` was printed | `0` |
| `clap` rejected the invocation | `2` |
| the input was rejected before anything ran | `2` |
| the user cancelled a prompt or the form | `130` |
| the command ran and failed (`--explode`) | `1` |

The numbers are the application's; `clifacet::contract::ExitPolicy::standard()` is the
mapping in one place, and `EXIT` in `src/exit.rs` is that policy named.

## Layout

One file per command, and one file per concern of the application around it:

```text
src/commands/mod.rs      the registry: one `mod` line per command
src/commands/example.rs  the command: `Cli`, `ExampleInput`, `Example`
src/interaction.rs       the interaction flags and the mode `POLICY`
src/exit.rs              the exit codes, and the one place an outcome is written
src/entry.rs             `run`, `main` and the scriptable entry points
src/form.rs              the full-screen form and its submission
src/lib.rs               the crate docs, the module list and the re-exports
src/main.rs              the `main` a real binary writes: args in, streams, exit code
tests/                   what proves each promise — the tables, the scenarios, the properties
```

`src/commands/example.rs`, top to bottom: `Cli` (the `clap` surface), `ExampleInput`
(validated input, built by the `TryFrom` bridge the derive calls), then `Example` (the
command, which touches no stream). Nothing in it knows about streams, modes or exit
codes; those are the files beside `commands/`. A second command would be a second
file in `commands/` and a tree in `commands/mod.rs`, the way `examples/backlog-lite`
does it. `lib.rs` re-exports everything public, so a caller never names a module.

## What tests it

| Test | The promise |
| --- | --- |
| `tests/example_command.rs` | the entry points, driven through buffers |
| `tests/validation_matrix.rs` | every layer's value for a given argv, as a table |
| `tests/facet_bridge.rs`, `tests/facet_bdd.rs` | the derive and the validated-input bridge |
| `tests/outcomes.rs`, `tests/outcomes_bdd.rs` | streams and exit codes |
| `tests/prompting_bdd.rs` | a question fills a gap and never re-asks a value |
| `tests/interaction_policy.rs` (library), `tests/interaction_policy_bdd.rs` | which mode runs, and why |
| `tests/tui_*.rs` | the form, its escape hatches and cross-mode parity |
| `tests/parity_bdd.rs`, `tests/harness_parity.rs` | every mode reaches the command with the same input |
| `tests/properties.rs` | the invariants no table would enumerate |
| `tests/features/*.feature` | the same promises as Given/When/Then |

Run them with `pixi run --frozen test` from the repository root. The scenarios are
written against the public entry points — `main`, `main_with`, `run_tui` — never
against a helper that already knows the answer.
