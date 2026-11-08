---
id: CLI-46
title: 'Run an application through clifacet::App'
status: In Progress
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 11:25'
labels: []
milestone: m-12
dependencies: []
type: task
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 1. Today every application writes the same pipeline by hand — parse, choose a frontend, ask, validate, run or preview, render, map the exit code; backlog-lite's app.rs is 120 lines of it. The runner owns that pipeline so main.rs is one call, and it is where the application context (CLI-47) and settings (CLI-51) are built later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `clifacet::run::<Cli>()`, and `clifacet::App::<Cli>::new()` with `.policy(..)` and `.run()`, own parse → ask → validate → run → render → exit, with exit codes from decision-3's contract
- [ ] #2 `clifacet::Globals` gives every application `--yes` and `--dry-run`, wired to the interaction policy and to a planned command's preview; an application can add its own global options beside them
- [ ] #3 `run_with(argv, capabilities, frontend, out, err)` runs the same pipeline in a test without a terminal or the process environment
- [ ] #4 A `Render` trait decides how a command's output is written, with a default for `Display` output; failures still go through `report::render`
- [ ] #5 Scenarios in crates/clifacet/features describe the pipeline: `--help` runs nothing, a dry run changes nothing, a declined question exits with the cancellation code
- [ ] #6 examples/simple's main.rs is one call to the runner and its tests and scenarios pass unchanged
- [ ] #7 The authoring guide has a runner section with a snippet check-docs compiles
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design agreed with the maintainer (2026-11-08, ask_user), before any production code:

1. **Which command runs.** `run::<Cli>()` and `App::<Cli>::new()` run the parsed `Cli` itself (`Cli: Command + Clone`), as a decision-14 tree already runs a clone of itself and as `commands!` (CLI-33) will generate. `.command(&other)` runs another command with the same input — how examples/simple keeps `Example` and its tests keep a recording command. Borrowed, because `Command` has no `&C` impl and cannot get one beside the `Planned` blanket.
2. **`--dry-run` on a plain command.** `Command` gains a provided `preview(&self, input) -> Result<Plan, Failure>` that refuses (validation: nothing ran). The `Planned` blanket returns `self.plan(input)`, so planned commands and `planned` trees preview without new code. Non-breaking for existing implementations.
3. **Render.** A strategy on the App: `trait Render<T> { fn render(&self, output: T, out: &mut dyn Write) -> io::Result<()> }`, default `Plain` for `Display` output, closures implement it, `.render(..)` swaps it.
4. **Full-screen mode.** Behind `tui`, the runner hosts `FormState::of::<Cli>()` on the diagnostic stream and submits through the same parse → validate → run/preview → render path; examples/simple's form pipeline moves into the library.

Decided without asking, as consequences of the above:
- `Globals` is `clap::Args` (`--yes`, `--dry-run`, both `global = true`) that an application flattens. The runner reads them from clap's matches by id, so no trait is needed and an application that already declares its own `yes`/`dry_run` flags keeps working.
- The mode the user asked for comes from `.mode_from(fn(&Cli) -> policy::Request)`, default none; `--yes` is ORed into it. Default policy `Policy::up_to(Mode::Interactive)`: prompts when there is a terminal, never for a script.
- A policy refusal is a validation failure (nothing ran), as in examples/simple. A resolved TUI without the `tui` feature is refused the same way.
- `run_parsed(cli, globals, ..)` is the pipeline after parsing, for an application (and examples/simple's `run(cli, ..)`) that already holds a `Cli`.
<!-- SECTION:NOTES:END -->
