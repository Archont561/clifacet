# AGENTS.md — Clifacet working rules

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

The launcher uses Git and the trusted `sandbox/developer-linux-64` branch and
may replace the local `.pixi` environment. A first restore requires the branch
to have been published or transferred into the local Git repository. If restore
fails, or Pixi is still unavailable, report the failure and ask for the missing
prerequisite; do not silently download or switch toolchains. On shared hosts,
ask before registering user tools or modifying an existing environment.

## Use the Pixi task surface

Run checks and tests through Pixi, not host-installed tools:

```sh
pixi run --frozen quality
pixi run --frozen test-src
pixi run --frozen test-integration
pixi run --frozen test-doc
pixi run --frozen test-scripts
pixi run --frozen coverage
pixi run --frozen sandbox-check
pixi run --frozen prose-check
```

Use `pixi run --frozen -x cargo ...` for a focused Cargo command where no
project task exists. Report checks that could not run; do not claim success.

## Starting without chat history

When the chat has no history, or the available history is insufficient to
understand the work, read `.agents/skills/session/SKILL.md` **before deciding
what to do**. Reconstruct only what the current request requires, preserve
existing changes, and do not invent missing history. Follow the restore rule
above before running checks or tests.

## New feature work

Every new feature begins by reading three skills, in this order, **before any
production code is written**:

1. [`.agents/skills/tdd/SKILL.md`](.agents/skills/tdd/SKILL.md) — the loop: one
   focused failing test, establish why it fails, the smallest change that passes,
   then refactor without changing behavior.
2. [`.agents/skills/bdd/SKILL.md`](.agents/skills/bdd/SKILL.md) — the workflow: a
   user-visible promise described as Given/When/Then with `rstest-bdd`, stored
   in the crate's `features/` directory and driven through public seams.
3. [`.agents/skills/clean-code-principles/SKILL.md`](.agents/skills/clean-code-principles/SKILL.md)
   — the design: clear names, cohesive modules, explicit errors, one source of
   truth, presentation separated from execution, no speculative abstraction.

This is mandatory, not "as relevant." A feature is not ready when it cannot point
at the failing test that came first, the scenario that describes the behavior it
adds, and the design decision that kept the change proportionate. Record that
evidence in the task notes.

Two narrowings, both of which have to be stated rather than assumed:

- A feature with no user-visible workflow says so in the task and explains why a
  scenario would not describe anything real; it does not silently skip BDD.
- A mechanical change with no behavior — a rename, a dependency bump, a docs or
  CI edit — may take the exception, and the task records that it did.

`test-local` is the loop's gate: it must be green before a feature is claimed
done, and a red test is never turned into a green one by weakening it.

## Agent skills

Project-owned skills live in `.agents/skills/`; read the matching `SKILL.md`
when its trigger applies. `tdd`, `bdd` and `clean-code-principles` are not
optional for feature work — see the section above. The `session` and
`regain-direction` skills are local, not downloaded or overwritten at session
start. Use the `backlog`, `audit`, `refactor`, `grill-me`, `writing-for-agents`,
and `skill-creator` skills as relevant. Do not install or update unreviewed
remote skills, or overwrite local skills, without maintainer authorization.

## The board: close the card when the work is done

A task's status is checked, not remembered. `backlog-check` fails when a card in
`Done` has an unchecked acceptance criterion, and when a card with every
criterion checked is still in `To Do` or `In Progress`.

The last step of a task is therefore not the green CI run — it is checking the
criteria and setting the status, in the same change as the work. When a criterion stays
open because its proof belongs to another card, record the proof here and say
where it is; that is what CLI-38 needed and did not have.

## Regain direction

When unsure about the goal, priority, or next step, read and follow
[the local regain-direction skill](.agents/skills/regain-direction/SKILL.md)
before continuing. If an important decision is unresolved, ask the maintainer
rather than silently broadening scope.

## Prose: two tools, two questions

The repository has two linters, and a finding belongs to exactly one of them (decision-20):

| Tool | Owns | Configuration | Run it |
| --- | --- | --- | --- |
| alint | what the repository *is*: required files, workspace membership, pinned workflows, generated sandbox files | `.alint.yml` | `pixi run --frozen alint` |
| Vale | how the guides *read*: spelling, terminology, editorial style | `vale.toml` (which renders `.vale.ini` and `.vale/styles/`) | `pixi run --frozen prose-check` |

After changing `README.md`, `AGENTS.md` or anything in `docs/`, run `prose-check`. After
changing a manifest, a workflow or a required file, run `alint`. `test-local` and `quality`
run both.

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

Reading is almost unrestricted: `gh run list`, `gh run view`, `gh run watch`, `gh run view
--job=<id>` and `gh pr checks` all work, so a run can be watched to completion even when it
had to be started by hand. `gh run view --log` does not: raw logs are served from
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

- `pixi-sandbox.toml` is the authored sandbox policy. Regenerate the publisher,
  relock workflow and restore script with `pixi-sandbox init` via Pixi; never
  hand-edit generated files. Validate with `pixi run --frozen sandbox-check`.
- Preserve the UI-free command core and the shared mode/validation boundary;
  consult backlog decisions before changing architecture.
- Do not publish, push, discard changes, close tasks, or alter scope without
  appropriate maintainer authorization.
- Do not automatically accept new Alint baseline findings or broaden the
  documented RustSec exception.
