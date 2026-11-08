---
id: CLI-46
title: 'Run an application through clifacet::App'
status: To Do
assignee: []
created_date: '2026-11-08 01:59'
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
