---
id: CLI-29
title: Discover and dispatch plugin binaries
status: To Do
assignee: []
created_date: '2026-11-01 22:22'
updated_date: '2026-11-01 22:46'
labels: []
milestone: m-8
dependencies: []
type: task
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A `<cli>-<name>` binary on PATH should be exposed as `<cli> <name>`, the way `git-lfs` becomes `git lfs`. Design and the measured clap behaviour: decision-17.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `plugins::discover(prefix, paths)` finds executables named `<prefix>-<name>`, never executes one, and reports names it dropped because a built-in has that name.
- [ ] #2 `plugins::append(&mut Command, &plugins)` adds one passthrough subcommand per plugin, with the resolved path in its `about`, and arguments forwarded verbatim.
- [ ] #3 `plugins::selected` routes a plugin invocation before the derive sees it, and `plugins::run` passes the plugin's exit status through with inherited stdio.
- [ ] #4 Discovery is explicit and directory-driven: nothing is scanned implicitly, and no plugin is probed with `--help`.
- [ ] #5 Windows is covered: `PATHEXT` extensions, case-insensitive names, and a platform job that runs the tests.
- [ ] #6 The forwarding rules (`--` consumed once, `--help` belonging to the plugin, exit status passthrough) are documented with a test each.
<!-- AC:END -->
