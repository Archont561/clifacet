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

- **A decision is data.** `Request` is validated input, `invocation_of` turns it into an
  `Invocation` (argument vector plus one sentence), and `Backlog`, a clifacet `Planned`
  command, wraps that in a `Plan` whose work is `run`. `--dry-run` describes the plan and
  a real run executes it; the library makes executing the plan the only way to run a
  planned command, so the two cannot disagree.
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
src/commands/mod.rs     the map: one `commands!` line, then the request and the runner's plan
src/commands/list.rs    one leaf per file: its flags, its input, its questions, its argv
src/commands/show.rs      (and new, start, done, search, doctor)
src/commands/task.rs    what show, start and done share: a task ID and moving it
src/status.rs           the `Status` every spelling of a status parses to
src/invocation.rs       a request as a command line, and the one place it runs
src/app.rs              modes, `--dry-run`, streams and exit codes
src/lib.rs              the crate docs, the module list and the re-exports
```

Adding a leaf is a new file, its name in the `commands!` list in `commands/mod.rs`, and one
arm in `invocation_of` — which the compiler demands, because the generated `Action` gains a
variant. `commands!` writes the `mod` lines, the re-exports, the `Commands` tree, `Action`
and the root `Cli` from that one list (decision-19 item 3), so the list is still the map of
the CLI a reviewer reads; it is just the only copy of it.

## Running it

```sh
cargo build -p clifacet-backlog-lite      # target/debug/bl
CLIFACET_BACKLOG_BIN="bun x --bun --no-install backlog" target/debug/bl list --limit 3
```

`CLIFACET_BACKLOG_BIN` is a command with whitespace-separated arguments and no shell
expansion — that is what makes it usable in a repository that runs the CLI through Bun, and
what keeps a test from ever spawning the real tool.

## Tests

`tests/flows.rs` asserts the argument vector byte for byte for each leaf, plus the flows
that are the reason to have a wrapper at all: the default verb, the asked-for title, the
declined confirmation, `--yes` with and without a terminal, and the status chosen from a
list.
