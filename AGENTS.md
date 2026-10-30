# AGENTS.md — Clifacet working rules

## Bootstrap first

Before running any check or test, run the bootstrap script once:

```sh
bash scripts/bootstrap.sh
export PATH="$HOME/.local/bin:$PATH"
```

It fast-forwards a clean checkout to its upstream (refusing when there is local
work to lose), restores the sandbox when the Pixi environment is missing or
`pixi` is not on `PATH`, installs the Git hooks, and ends with `test-scripts` and
`alint` as a smoke check. `--skip-restore`, `--skip-hooks`, `--skip-checks` and
`--no-fetch` narrow it.

To restore without the checkout step, run the generated launcher directly:

```sh
bash scripts/restore.sh
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
   with the crate's tests and driven through public seams.
3. [`.agents/skills/clean-code-principles/SKILL.md`](.agents/skills/clean-code-principles/SKILL.md)
   — the design: clear names, cohesive modules, explicit errors, one source of
   truth, presentation separated from execution, no speculative abstraction.

This is mandatory, not "as relevant". A feature is not ready when it cannot point
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

## Regain direction

When unsure about the goal, priority, or next step, read and follow
[the local regain-direction skill](.agents/skills/regain-direction/SKILL.md)
before continuing. If an important decision is unresolved, ask the maintainer
rather than silently broadening scope.

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
