# backlog-lite

A small, friendly `bl` wrapper over the [Backlog.md](https://github.com/MrLesk/Backlog.md)
CLI, built on clifacet's published API. It exists for two reasons:

- it is a **worked example** of a wrapper around somebody else's CLI — the second example in
  this repository, and the first that does not own the tool it drives;
- it is the **acceptance case** for decision-18: every authoring feature that decision
  proposes was written by hand here first.

Nothing is reimplemented. Every leaf builds an argument vector; one function spawns it.

```text
bl                              # list — the thing you do most
bl list --status                # no value: pick the status from a list
bl --json show CLI-20           # one --json for the whole run
bl new "Ship it" --ac one       # the title is asked for when missing
bl done CLI-20                  # confirmed; --yes says so in advance
bl --dry-run done CLI-20        # print the exact command, run nothing
```

## What the wrapper adds

| Concern | Backlog.md | `bl` |
| --- | --- | --- |
| Machine output | `--json` on each leaf | `--yes`-style global `--json`, forwarded once |
| Human output | `--plain` on `view`/`list` | `--plain` by default: a wrapper is a pipe |
| Status values | `--status "In Progress"` | `--status in-progress`, validated against the real words; a bare `--status` asks which |
| Moving a task | `task edit ID --status Done` | `done ID`, confirmed |
| Missing title | `task create ""` succeeds untitled | asked for; with no terminal it fails |
| Preview | none | `--dry-run` prints the runnable line |

## Design notes

- **The application is three things.** A context (`Program`: the command to forward to), a
  `PlannedWith` impl (`Backlog`: a request on that program as a `Plan` whose work is `run`),
  and `run_app()`. Everything else — parse, mode, ask, validate, preview, run, render, exit
  code — is clifacet's `App` (CLI-46/CLI-48), so `main.rs` is one call.
- **A decision is data.** `Request` is validated input, `invocation_of` turns it into an
  `Invocation` (argument vector plus one sentence), and `Backlog` wraps that in a `Plan`
  whose work is `run`. `--dry-run` describes the plan and a real run executes it; the
  library makes executing the plan the only way to run a planned command, so the two cannot
  disagree.
- **Questions never change an explicit answer.** Every prompt goes through `Session`, which
  returns a supplied value untouched, and builds no frontend at all when the plan asks
  nothing.
- **An unanswerable confirmation is not a confirmation.** `Session::confirm` says how it was
  settled, and the wrapper matches on it: `Unavailable` reads "pass `--yes`" instead of
  "declined", and `Assumed` proceeds with or without a terminal. Nothing inspects the plan.
- **A filter nobody named is a choice, not a guess.** A bare `--status` asks with
  `Session::choose`, offering exactly the statuses there are; where nothing can ask, it is
  refused as incomplete rather than read as "no filter".
- **Somebody else's exit codes are somebody else's.** The child's status is returned
  unchanged. Only the wrapper's own failures go through `ExitPolicy`.

## Layout

One file per command, and one file per layer around them:

```text
src/commands/mod.rs     the request, its conversions, and what the folder exports
src/commands/list.rs    one leaf per file: its flags, its input, its questions, its argv
src/commands/show.rs      (and new, start, done, search, doctor)
src/commands/_task.rs   what show, start and done share: a task ID and moving it
src/status.rs           the `Status` every spelling of a status parses to
src/invocation.rs       a request as a command line, purely
src/program.rs          the context: the program forwarded to, and the one place it spawns
src/lib.rs              the command, the wiring, and `run_app`
src/main.rs             one call to `run_app()`
```

Adding a leaf is a new file and one arm in `invocation_of` — which the compiler demands,
because the generated `Action` gains a variant. The `#[clifacet::registry]` line on
`mod commands` in `src/lib.rs` is the whole registration: it writes the `mod` lines, the
re-exports, the `Commands` tree, `Action` and the root `Cli` from the report `build.rs`
wrote, and includes the folder's own `mod.rs` beside them (CLI-58), so the files in the
folder are the map of the CLI.

## Running it

```sh
cargo build -p clifacet-backlog-lite      # target/debug/bl
target/debug/bl list --limit 3
```

The program it forwards to is the application's **context**, not a setting: `main` passes
`Program::default()` — `backlog` on the `PATH` — and a test passes its own fake. Running the
CLI through Bun instead is a one-word change in `Program::new(["bun", "x", "--bun",
"backlog"])` in the binary, which also compiles no path from the environment into what
runs.

## Tests

`tests/flows.rs` asserts the argument vector byte for byte for each leaf, plus the flows
that are the reason to have a wrapper at all: the default verb, the asked-for title, the
declined confirmation, `--yes` with and without a terminal, and the status chosen from a
list. `tests/program.rs` proves the program comes from the context — a fake in, the fake's
name out — and `features/*.feature` drive the same seams a person does, through `App`.
