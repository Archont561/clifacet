---
id: CLI-29
title: Document a plugin recipe for <cli>-<name> binaries
status: To Do
assignee: []
created_date: '2026-11-01 22:22'
updated_date: '2026-11-06 14:31'
labels: []
milestone: m-8
dependencies: []
type: task
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-17 was rejected as a library API on 2026-11-06 and a documented recipe accepted instead: a `<cli>-<name>` binary exposed as `<cli> <name>`, the way `git-lfs` becomes `git lfs`, built by the application from `clap` and `std` using the behaviour decision-17 measured. A plugin is outside what clifacet guarantees — its arguments are not validated, it is outside the mode contract and parity — and the recipe says so instead of hiding it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The authoring guide has a plugin section whose snippet compiles under check-docs: discovery over directories the application names, never executing a candidate and never probing it with `--help`.
- [ ] #2 The snippet adds one passthrough subcommand per plugin with its resolved path in the help, drops (and reports) a name that collides with a built-in, and routes a plugin invocation before the derive converts.
- [ ] #3 A table states the forwarding rules — arguments forwarded verbatim, `--` consumed once by clap, `--help` belonging to the plugin, exit status passed through — and the Windows differences (`PATHEXT`, case-insensitive names, `.cmd`/`.bat` needing a shell).
- [ ] #4 The recipe refuses to launch a plugin when the run's plan is headless, and explains why a plugin is outside validation, the mode contract and parity.
- [ ] #5 Clifacet gains no `plugins` module, feature or dependency: the recipe uses only `clap` and `std`.
<!-- AC:END -->
