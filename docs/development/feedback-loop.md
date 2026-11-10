# The feedback loop

A change moves through four places where it can fail: the editor, the commit, the push
and CI. Each one costs more than the one before it, and each one is cheaper to fix than
the one after. This page records what each stage actually covers, what that costs, and
where the loop still leaks — with the measurements they come from.

The rule the whole design serves: **a failure belongs at the earliest stage that can see
it.** A check that runs in two places is not redundant; it is a check whose earlier copy
is the one a person experiences.

## The four stages

| Stage | What runs | Cost when green | What it cannot see |
| --- | --- | --- | --- |
| Editor | `cargo check`, rust-analyzer, clippy hints | continuous | anything outside the crate being edited |
| `pre-commit` | every formatter, linter and repository check | ~7 s | everything requiring a built test binary |
| `pre-push` | audit, deny, three test suites | ~45 s | the three-OS matrix, the MSRV, coverage |
| CI | the above, again, on three platforms | 1–9 min | nothing — this is the last word |

The numbers are measured on a warm checkout, which is the only state worth quoting: a
cold one measures the sandbox restore, not the check.

## What the hooks cover

`lefthook.yml` installs three hooks, and they are split by what a gate protects rather
than by how much they are trusted.

**`pre-commit`** runs the linters, because a commit is the last moment a fix is still
free. Every job is offline — the Vale styles are committed, `audit-offline` reads a local
clone when there is one, and each hook calls `pixi run --frozen --no-install` so a restored
sandbox never re-solves. It is parallel, and formatters re-stage what they rewrote, so a
commit is never half-formatted.

| Hook job | Time | Fails a commit for |
| --- | --- | --- |
| `rust-fmt` | 0.5 s | unformatted Rust |
| `rust-clippy` | 1.6 s | a warning, or a manifest change that introduces one |
| `feature-boundaries` | 2.3 s | a `#[cfg]` mistake in any optional feature |
| `check-docs` | 1.0 s | a dead link, a stale anchor, a snippet that does not compile, a documented name that no longer exists |
| `prose` | 0.6 s | a Vale finding, or a generated file that is not current |
| `alint` | 0.4 s | a repository policy violation no other linter reads |
| `actionlint`, `sandbox-check`, `skills-check` | 0.3 s | a workflow syntax error, a hand-edited generated workflow, a drifted skill lock |

Seven seconds, parallel, and it is the difference between a commit and a commit that
costs a CI run.

**`commit-msg`** runs `convco`. The changelog, the version bump and the release notes are
all derived from commit messages, so this hook protects a data source, not a style.

**`pre-push`** runs the advisory gates and the three test suites — ~45 s locally,
sequential and fail-fast (`piped`), because the jobs share one `target/` directory and
there is no point running 732 tests after Clippy has already said no. This is the same
work CI is about to do, run once per push instead of once per commit.

### Using it

Install the hooks once, through the script that restores the environment:

```sh
bash scripts/bootstrap.sh
```

Then work normally. The loop that keeps a change cheap:

```sh
pixi run --frozen --no-install automation gates   # what CI will run, offline, in its order
git commit                                        # the same checks, seconds, on the files touched
git push                                          # the suites, once
```

`automation gates` is the gate to run before asking for a review: it is the offline subset
of CI in CI's order, so it fails the way CI would fail, in about a minute instead of nine.
`automation gates-online` adds the two RustSec checks that need the network.

Two habits that break the loop, both enforced: committing with `--no-verify` (CI's
`commit-messages` and `quality` jobs exist precisely for whoever does), and hand-editing a
generated file (`CHANGELOG.md`, `README.md`, the two sandbox workflows) instead of editing
what renders it.

## Why CI is slow

A full CI run takes 1–9 minutes, and the range is the finding. Measured runs:

| Run | Change | Wall clock |
| --- | --- | --- |
| 42740105243 | documentation only | 1 min |
| 42739589925 | documentation only | 1 min |
| 42738018113 | full | 9 min |
| 42736580849 | full | 7 min |

The documentation lane is what a fast loop looks like: one job, one Pixi environment, and
`Documentation checks` finishes in about 40 s. The full run is dominated by four jobs that
each rebuild the workspace from scratch:

| Job | Duration | Why it is the long pole |
| --- | --- | --- |
| `Supported platforms (windows-latest)` | 9 min | full workspace build, no compiled-artifact cache |
| `Supported platforms (macos-latest)` | 5 min | same |
| `Supported platforms (ubuntu-latest)` | 4 min | same |
| `Tests and source coverage` | 4 min | the suites again, instrumented |

Three causes, in order of cost:

1. **No compilation cache.** The `platforms`, `coverage` and `msrv` jobs cache
   `~/.cargo/registry` — downloads only — so every job recompiles all dependencies on every
   run. The Pixi cache helps the environment, not the build. This is most of the nine
   minutes and all of the variance.
2. **Every job installs an environment.** Six jobs repeat `Set up Pixi` before doing any
   work. It is cached and it is fast, but it is six copies of the same 20–30 s.
3. **The platform matrix is unconditional.** Windows is the slowest runner and the least
   likely to fail for the changes this repository usually makes, yet it runs on every
   push that touches a `.rs` file.

### Changing it

The documentation lane is the pattern to extend, not an exception to preserve. In
increasing order of what they buy:

- **Cache the build.** An `actions/cache` entry on `~/.cargo` and `target/`, keyed on
  `Cargo.lock` and the toolchain, turns a repeat run's rebuild into a link. This is the
  single largest win and it needs no workflow restructuring.
- **Classify further than documentation.** `scripts/change_scope.py` already decides the
  lane and is tested; a "Rust-only, no manifest, no public API" class does not need the
  MSRV job, which only moves when the manifests do.
- **Run the platform matrix on the merge, not the push.** `main` still sees three
  operating systems — that is what makes a merge trustworthy — but a branch's push sees
  Linux and the pull request gets the rest.
- **Split the suites.** `Tests and source coverage` runs the nextest suites and then the
  instrumented run. Coverage on every push is a report nobody reads between pushes.

Measured: a warm local `pre-commit` is 7 s, `pre-push` is 45 s, and the offline gate is
about a minute. A CI run that adds nine is not buying nine minutes of information over
those.

## Where the loop still leaks

Honest limits of the current setup, each of which has cost a run:

- **The coverage floor and the three-OS matrix are CI-only.** No local command reproduces
  them; a green local gate and a red CI are possible, and that is a design choice — the
  alternative is nine minutes on every commit.
- **RustSec needs the network.** The hooks run the offline subsets and step aside; CI runs
  the fetching audit. An advisory published between a push and a run is caught only by the
  run.
- **`CHANGELOG.md` must be regenerated after the commit that changes it**, so a `feat:`
  needs a second commit before `changelog-check` can pass. The hook cannot see this — the
  commit does not exist yet when it runs — so it surfaces in CI.
- **A restored sandbox has no `node_modules`** until `bun-install`. Markdownlint, the README
  renderer and the backlog board all fail with an unhelpful message until it runs once.
- **CI job logs are unreadable from an agent sandbox**, so a red run is diagnosed through
  `gh run view --json` and the check-run annotations rather than the log text.
