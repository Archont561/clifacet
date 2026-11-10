# AGENTS.md

> Guidance for AI coding agents working in this repo. Read this first.
> **Before building, changing, or debugging anything: verify live state** (read the actual
> code, config, and deployed branch). Do not act on stale assumptions.
>
> **Generated and maintained by
> [@eugeniughelbur/agents-md](https://www.npmjs.com/package/@eugeniughelbur/agents-md).**
> The regions between `agents-md:begin` and `agents-md:end` are refreshed by
> `pixi run agents`; everything outside them — including `## Notes` — is hand-written and
> preserved. Keep a durable rule outside the markers; keep a *detected* fact inside them.

<!-- agents-md:begin id=overview -->
## Overview

Clifacet is a Rust library for command-line applications whose commands are **functions**:
a `#[clifacet::command]` function's parameters are its arguments, and the same declaration
gives the `clap` surface, the validated input, the prompts and the full-screen form. An
application is a value — `clifacet::App` — that composes the policy, the settings layering
and the exit codes around it.

The repository is a Cargo workspace of two published crates (`crates/clifacet`,
`crates/clifacet-macros`) and three consumer examples (`examples/simple`,
`examples/discovered`, `examples/backlog-lite`), plus Python automation, a handful of
pinned npm tools, and **one Pixi environment**, which is the only supported way to run
anything here.

The tools detect the stack as Rust (workspace, edition 2024, MSRV 1.88), Python 3.15
(automation) and Node tooling from `package.json`. The detected "node" classification above
is an artifact of that file existing; the product is Rust. Why all four exist is in
[docs/architecture/repository.md](docs/architecture/repository.md).
<!-- agents-md:end id=overview -->

<!-- agents-md:begin id=commands -->
## Commands

Everything runs through Pixi. `--frozen --no-install` reuses the restored environment
instead of re-solving it, and it is what the hooks and CI use.

```sh
bash scripts/bootstrap.sh                          # environment + git hooks + smoke check
pixi run --frozen --no-install automation list    # every task, with what it covers
pixi run --frozen --no-install automation gates   # the offline gate, cheapest failure first
pixi run --frozen automation gates-online         # adds the two RustSec checks
pixi run --frozen --no-install hooks-install      # pre-commit, commit-msg, pre-push
```

The graph is `dodo.py`; `automation/lib/tasks.py` bridges it to `scripts/*.py`, and every
script still runs on its own (`pixi run --frozen python scripts/check_docs.py`).

| Task | What it answers |
| --- | --- |
| `automation gates` | everything a change must pass, offline, in CI's order |
| `automation docs` | links, anchors, compiled snippets, real API names |
| `automation prose` | the rendering is current, then Vale |
| `automation markdownlint` | the structure of every page |
| `automation test` | nextest, doctests and the Python suite |
| `automation coverage` | LCOV plus the 90% source-line floor |
| `automation fmt` / `lint` | rewrite / check the Rust |
| `automation readme` / `readme_check` | render README.md / fail when it drifted |
| `automation agents` | refresh the marked regions of this file |
<!-- agents-md:end id=commands -->

<!-- agents-md:begin id=testing -->
## Testing

`automation gates` is the gate; `automation test` is the suites inside it.

| Suite | Command | Size at 0.5.0 |
| --- | --- | --- |
| Rust unit + integration | `cargo nextest run --workspace --all-features --locked` | 732 tests |
| Doctests | `cargo test --workspace --all-features --doc` | nextest does not run them |
| Documented snippets | `cargo test -p clifacet-example --test documented_snippets` | 25, generated from the pages |
| Python automation | `python -m pytest scripts/tests` | 463 tests, 2 skipped |

A change is not done when the tests pass — it is done when the criteria on its backlog card
are checked and the card's status says so (see *The board* below). Tests come first for
feature work: see *New feature work*.

Never make a red test green by weakening it. A property test that was narrowed, an
assertion that was loosened, or a case that was deleted all need a reason in the commit.
<!-- agents-md:end id=testing -->

<!-- agents-md:begin id=structure -->
## Project structure

```text
crates/clifacet/          the library: command, facet, policy, prompt, tui, contract, report,
                          harness, secret, provenance, settings, build, app
crates/clifacet-macros/   the Facet derive, #[clifacet::command], #[clifacet::registry], version!
examples/                 consumers of the published API — never a private path
  simple/                 the reference application; its own README and suite
  discovered/             src/commands/ registered from build.rs
  backlog-lite/           a wrapper around somebody else's CLI: the acceptance case
automation/               the doit bridge (lib/tasks.py)
scripts/                  the implementations: one main(argv) -> int each
dodo.py                   the task graph
docs/                     the knowledge layer — guides/ concepts/ reference/ architecture/ development/
backlog/                  decisions, milestones, tasks (planning, not documentation)
.readme/                  the README template; README.md is rendered from it
```

`examples/` are workspace members with their own manifests on purpose: an example inside
the library could use `pub(crate)` items and keep compiling after a change that breaks
every external caller.
<!-- agents-md:end id=structure -->

<!-- agents-md:begin id=conventions -->
## Code style & conventions

- **Formatter and linter are the tools'**: `cargo fmt` and `cargo clippy -D warnings` over
  `--workspace --all-targets --all-features`. Clippy's `pedantic` group is on at `warn`.
- **The command core stays UI-free.** A command takes validated input and returns data or a
  failure; it never writes a stream, reads a terminal or chooses an exit code.
- **One declaration per fact.** A parameter's flags, prompt, form field and rules go in the
  `#[clifacet(..)]` beside it — never a second list that can drift.
- **Prose is American English**, present tense, second person. Vale owns the words, alint
  owns the files, markdownlint owns the structure, `check_docs.py` owns the links and the
  snippets: a finding belongs to exactly one of them.
- **A doc comment is documentation**: `# Errors` and `# Panics` sections where they apply.
- **Python automation never runs at import time**; every module keeps `main` and an
  `if __name__ == "__main__"` guard so it stays runnable on its own.
- **Comments explain why.** A comment that restates the code is noise; a comment that
  records a decision is the only copy of that decision.
<!-- agents-md:end id=conventions -->

<!-- agents-md:begin id=git -->
## Git & PR workflow

Default branch `main`; remote `https://github.com/Archont561/clifacet`.

- **Conventional Commits**, enforced by `convco` on commit-msg and by the
  `Conventional commit messages` job. `CHANGELOG.md` is *generated* from them — edit the
  commit message, never the file.
- **`CHANGELOG.md` must be regenerated after the commit that changes it**:
  `pixi run --frozen changelog`, then a second commit. A PR that adds a `feat:` fails
  `changelog-check` until that second commit exists.
- **Publishing is a separate, authorized act.** A merge never publishes. The release is a
  manual `release.yml` dispatch from `main` against the `crates` environment, with a
  rehearsal (`dry_run` left true) first. Runbook:
  [docs/development/development.md](docs/development/development.md#release).
- **Versions live in three manifests** and `pixi run bump-version --check` proves they
  agree. Bump with `bump-version X.Y.Z --relock`, then `pixi run readme` so the rendered
  README names the new version.
<!-- agents-md:end id=git -->

<!-- agents-md:begin id=gotchas -->
## Gotchas & hard-won lessons

Each of these cost a CI run at least once. Where a check was possible, there is one.

- **`None` as `argv` reads the wrong command line.** A script's `main(argv)` under doit gets
  doit's own `sys.argv`; always pass a list, even an empty one.
- **A Python action's return value is doit's result.** A `main` returning `1` would pass;
  raise instead, which is what `automation/lib/tasks.py` does.
- **No `file_dep` or `targets` on a check.** Doit would skip it, and a skipped check reports
  success without looking.
- **`pixi run <task>` after a restore needs `bun-install`** before anything that reads
  `node_modules`: markdownlint, the README renderer, the backlog board, this file's tool.
- **The README generator prompts before overwriting `README.md`** and has no flag to skip
  it, so `scripts/readme.py` renders in a scratch directory and copies the result in.
- **`agents-md` rewrites its marked regions from detection on every run**, and it detects
  this repo as `node`, so a refresh reintroduces stubs in those sections. Fill them again
  and review the diff; keep durable rules outside the markers.
- **trybuild rewrites path-like text** in compile-fail output. Keep paths out of messages.
- **A shallow clone breaks `changelog`**: `git fetch --unshallow origin` first.
- **CI job logs are unreadable from this sandbox** (`EOF`). Use `gh run view --json` step
  lists and `gh api .../check-runs/<id>/annotations` instead.
<!-- agents-md:end id=gotchas -->

<!-- agents-md:begin id=security -->
## Security & secrets

- **No secrets in the repository.** crates.io publishing uses OIDC trusted publishing: the
  workflow exchanges a GitHub token for a short-lived registry token, so there is no
  `CARGO_REGISTRY_TOKEN` here to leak.
- **A credential is a `Secret<String>`**, never a `String`. `Debug` and `Display` both print
  `[redacted]`, which is what keeps a diagnostic from printing one.
- **Never paste a token into chat, a commit, a log or a test fixture.** If `git` or `gh`
  fails with an authentication error, report it and ask for the connection to be
  re-established.
- **Vale's pinned packages are committed** under `.vale/styles/`; nothing here fetches a
  style at check time.
- **A pre-commit hook cannot see a CI-only failure.** The hooks cover the offline gate; the
  npm, RustSec and cross-platform checks are CI's, and claiming they passed locally is a
  false report.
<!-- agents-md:end id=security -->

<!-- agents-md:begin id=boundaries -->
## Boundaries

- ✅ **Always**: run `pixi run --frozen --no-install automation gates` before proposing a
  change; regenerate `CHANGELOG.md` after a commit that changes it; re-render the README
  after editing its template; update the page a behavior change affects; close the backlog
  card in the same change as the work.
- ⚠️ **Ask first**: raising the MSRV; adding a dependency (it re-solves `pixi.lock` or
  `bun.lock`); changing the public API or the command core's seams; editing
  `pixi-sandbox.toml`; touching `.github/workflows/release.yml`; broadening the documented
  RustSec exception; accepting a new alint baseline finding.
- 🚫 **Never**: commit secrets or API keys; edit a generated file by hand (`CHANGELOG.md`,
  `README.md`, `.vale.ini`, `.vale/styles/`, `scripts/restore.sh`, the two generated
  workflows, `documented_snippets.rs`); force-push `main`; publish, tag or dispatch a
  release without explicit maintainer authorization; weaken a test to make it pass.
<!-- agents-md:end id=boundaries -->

## Session start: check for Pixi

**At the start of every session, before any check, test or build, check whether `pixi`
exists:**

```sh
export PATH="$HOME/.local/bin:$PATH"
command -v pixi || echo "pixi is missing — restore the environment"
```

If `pixi` is missing, do this, in this order, and do not skip the hooks step:

```sh
bash scripts/bootstrap.sh          # restores the environment, then installs the hooks
export PATH="$HOME/.local/bin:$PATH"
```

`bootstrap.sh` is the whole sequence, and it is what to run rather than the pieces, because
it does them in the order that works:

1. **fast-forwards** a clean checkout that is strictly behind its upstream, and refuses
   everything else rather than losing work;
2. **restores the sandbox** when `pixi` is not on `PATH` or `.pixi` is missing, by running
   the generated launcher (`scripts/restore.sh`), then registers the user tools on `PATH`;
3. **installs the Git hooks** with `lefthook install`, so the commit and push gates are the
   ones in `lefthook.yml` and not whatever the checkout arrived with;
4. **proves the result** with `test-scripts` and `alint` as a smoke check.

`--skip-restore`, `--skip-hooks`, `--skip-checks` and `--no-fetch` narrow it. To restore
without the checkout step, run the generated launcher directly:

```sh
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"
```

Then install the hooks, because a restore does not do it and a missing hook is invisible
until the commit that should have been checked lands:

```sh
pixi run --frozen --no-install hooks-install
```

Confirm both before starting work — a session that skips this reports a missing tool as a
broken change, and a session without hooks reports nothing until CI does:

```sh
pixi --version                       # the environment is up
test -f .git/hooks/pre-commit && echo "hooks are installed"
```

The launcher uses Git and the trusted `sandbox/developer-linux-64` branch and may replace
the local `.pixi` environment. A first restore requires the branch to have been published
or transferred into the local Git repository. If restore fails, or Pixi is still
unavailable, report the failure and ask for the missing prerequisite; do not silently
download or switch toolchains. On shared hosts, ask before registering user tools or
modifying an existing environment.

## Starting without chat history

When the chat has no history, or the available history is insufficient to understand the
work, read [`.agents/skills/session/SKILL.md`](.agents/skills/session/SKILL.md) **before
deciding what to do**. Reconstruct only what the current request requires, preserve
existing changes, and do not invent missing history. Follow the restore rule above before
running checks or tests.

## New feature work

Every new feature begins by reading three skills, in this order, **before any production
code is written**:

1. [`.agents/skills/tdd/SKILL.md`](.agents/skills/tdd/SKILL.md) — the loop: one focused
   failing test, establish why it fails, the smallest change that passes, then refactor
   without changing behavior.
2. [`.agents/skills/bdd/SKILL.md`](.agents/skills/bdd/SKILL.md) — the workflow: a
   user-visible promise described as Given/When/Then with `rstest-bdd`, stored in the
   crate's `features/` directory and driven through public seams.
3. [`.agents/skills/clean-code-principles/SKILL.md`](.agents/skills/clean-code-principles/SKILL.md)
   — the design: clear names, cohesive modules, explicit errors, one source of truth,
   presentation separated from execution, no speculative abstraction.

This is mandatory, not "as relevant." A feature is not ready when it cannot point at the
failing test that came first, the scenario that describes the behavior it adds, and the
design decision that kept the change proportionate. Record that evidence in the task notes.

Two narrowings, both of which have to be stated rather than assumed:

- A feature with no user-visible workflow says so in the task and explains why a scenario
  would not describe anything real; it does not silently skip BDD.
- A mechanical change with no behavior — a rename, a dependency bump, a docs or CI edit —
  may take the exception, and the task records that it did.

`automation gates` is the loop's gate: it must be green before a feature is claimed done,
and a red test is never turned into a green one by weakening it.

## Agent skills

Project-owned skills live in `.agents/skills/`; read the matching `SKILL.md` when its
trigger applies. `tdd`, `bdd` and `clean-code-principles` are not optional for feature
work — see the section above. The `session` and `regain-direction` skills are local, not
downloaded or overwritten at session start. Use the `backlog`, `audit`, `refactor`,
`grill-me`, `writing-for-agents`, and `skill-creator` skills as relevant. Do not install or
update unreviewed remote skills, or overwrite local skills, without maintainer
authorization.

## The board: close the card when the work is done

A task's status is checked, not remembered. `backlog-check` fails when a card in `Done` has
an unchecked acceptance criterion, and when a card with every criterion checked is still in
`To Do` or `In Progress`.

The last step of a task is therefore not the green CI run — it is checking the criteria and
setting the status, in the same change as the work. When a criterion stays open because its
proof belongs to another card, record the proof here and say where it is; that is what
CLI-38 needed and did not have.

## Regain direction

When unsure about the goal, priority, or next step, read and follow
[the local regain-direction skill](.agents/skills/regain-direction/SKILL.md) before
continuing. If an important decision is unresolved, ask the maintainer rather than silently
broadening scope.

## Documentation: where a page goes, and what a change owes it

`docs/` is the product's knowledge layer, not a byproduct. Five directories, one purpose
each: `guides/` for tasks, `concepts/` for reasons, `reference/` for promises,
`architecture/` for how the repository is built, `development/` for working on it.
`docs/README.md` is the only page at the top level, and `scripts/check_docs.py` fails a page
filed anywhere else.

The policy is
[docs/concepts/documentation-policy.md](docs/concepts/documentation-policy.md) — read it
before changing behavior. In short: a fact has one canonical home and every other page
links to it; a change that makes a page wrong fixes the page or the review records why not;
an obsolete instruction is removed rather than annotated; and every command in the prose
was run. `README.md` is generated from `.readme/README.template.md` by
[readme-md-generator](https://www.npmjs.com/package/readme-md-generator), so a README
change edits the template and runs `pixi run readme`.

## Prose: two tools, two questions

The repository has four linters, and a finding belongs to exactly one of them
(decision-20):

| Tool | Owns | Configuration | Run it |
| --- | --- | --- | --- |
| alint | what the repository *is*: required files, workspace membership, pinned workflows, generated sandbox files | `.alint.yml` | `pixi run --frozen alint` |
| Vale | how the guides *read*: spelling, terminology, editorial style | `vale.toml` (which renders `.vale.ini` and `.vale/styles/`) | `pixi run --frozen prose-check` |
| markdownlint-cli2 | the *shape* of a page: heading levels, lists, fences, stray blank lines | `.markdownlint-cli2.yaml` | `pixi run --frozen markdownlint` |
| `check_docs.py` | whether a link resolves, a fence compiles, a documented path exists | the pages themselves | `pixi run --frozen check-docs` |

After changing `README.md`, `AGENTS.md` or anything in `docs/`, run `prose-check`. After
changing a manifest, a workflow or a required file, run `alint`. `gates` runs all of them.

Rules for changing the prose check:

- **Edit `vale.toml`, never `.vale.ini` or `.vale/styles/`.** They are generated;
  `prose-check` fails when they have drifted. Render them with `pixi run prose-sync`, which
  also re-installs the pinned packages.
- **A switched-off rule keeps its reason.** `prose-check` refuses a `NO` with no `reason`:
  the reason is the only record of a decision that is invisible in the generated INI.
- **Do not pin a package by name.** A name installs whatever is newest, so a release
  upstream would change what CI rejects without a commit here.
- **Fix the prose, do not silence the rule.** A new word for the project's own vocabulary
  goes in `accept` in `vale.toml`. Switching a rule off is for prose written a certain way
  on purpose, and it needs the reason.
- **Code is not prose.** A `rust` fence and an inline span are skipped, so an identifier is
  never "misspelled" and a guide may quote the API exactly. A finding inside a code block
  means the block is not fenced properly.
- **Vale does not run in every environment.** The sandbox cannot reach conda-forge or
  GitHub's release assets, so `prose-check` fails there for lack of the binary rather than
  for a finding. Report that, and let CI make the call; do not claim the prose passed.

## GitHub operations from an agent session

`git` and `gh` work here; two `gh` commands do not, and knowing which saves a detour:

| Command | Result | What to do instead |
| --- | --- | --- |
| `gh pr edit` | 403 from the app token | amend the commit message with `git commit --amend`, or describe the change in a PR comment |
| `gh run cancel` | 403 from the app token | push a fix on the same branch: the `ci` concurrency group cancels the superseded run |
| `gh workflow run` | 403 from the app token (`Resource not accessible by integration`) | a maintainer dispatches from the Actions tab, or from their own authenticated `gh`; record the run's address in the task afterwards |

Reading is almost unrestricted: `gh run list`, `gh run view`, `gh run watch`, `gh run
view --job=<id>` and `gh pr checks` all work, so a run can be watched to completion even
when it had to be started by hand. `gh run view --log` does not: raw logs are served from
`results-receiver.actions.githubusercontent.com`, which the sandbox cannot reach, so it
fails with `EOF`. Verify a run from its per-step run/skipped list and from repository state
(tags, releases) instead of from its raw log.

`gh run watch`, `gh pr checks --watch`, `gh pr merge` and `gh api` all work, including
reading an environment's protection rules:

```sh
gh api repos/Archont561/clifacet/environments/crates --jq '.protection_rules'
```

Never request credentials in chat; if `git` or `gh` fails with an authentication error,
report it and ask for the connection to be re-established.

## Ownership and safety

- `pixi-sandbox.toml` is the authored sandbox policy. Regenerate the publisher, relock
  workflow and restore script with `pixi-sandbox init` via Pixi; never hand-edit generated
  files. Validate with `pixi run --frozen sandbox-check`.
- Preserve the UI-free command core and the shared mode/validation boundary; consult
  backlog decisions before changing architecture.
- Do not publish, push, discard changes, close tasks, or alter scope without appropriate
  maintainer authorization.
- Do not automatically accept new Alint baseline findings or broaden the documented RustSec
  exception.

## Notes

<!-- Human-owned. Anything here is never touched by re-runs of agents-md. -->

The sections above the `## Session start` heading are generated or refreshed; everything
from there down is hand-written and untouched by the tool. A durable rule belongs here; a
fact the tool can detect from the repository belongs in a marked region, where a refresh
keeps it true.

---

<!-- Generated and maintained by agents-md (https://github.com/eugeniughelbur/agents-md).
     Content INSIDE agents-md:begin/end markers is regenerated on re-run.
     Everything OUTSIDE the markers (including ## Notes) is yours and is preserved. -->
