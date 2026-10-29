# AGENTS.md — Clifacet working rules

## Bootstrap first

Before running any check or test, check whether `pixi` is on `PATH`. If it is
not, run the repository's generated restore launcher first:

```sh
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"
command -v pixi
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

## Agent skills

Project-owned skills live in `.agents/skills/`; read the matching `SKILL.md`
when its trigger applies. The `session` and `regain-direction` skills are local,
not downloaded or overwritten at session start. Use the `backlog`, `tdd`, `bdd`,
`audit`, `refactor`, `clean-code-principles`, `grill-me`, `writing-for-agents`,
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
