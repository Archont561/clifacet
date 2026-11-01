---
id: CLI-33
title: Register one command per file
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
labels: []
milestone: m-7
dependencies:
  - CLI-28
type: task
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A command lives in its own module with a readable registration line, and the registry is explicit. Decision-15 rejects run-time dispatch for commands because an external subcommand hands back untyped OsStrings, which would break the rule that every value a command receives is validated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 A command lives in its own module and is registered by a line a reader can follow without reading a macro.
- [ ] #2 The registry is explicit: nothing is discovered globally and nothing is registered by initialising a static.
- [ ] #3 Registering two commands with the same name fails to compile rather than resolving by order.
- [ ] #4 Dispatch stays compile-time, so no subcommand arrives as an untyped `OsString` that skipped validation.
- [ ] #5 The reference application registers its commands file by file, with its tests unchanged.
<!-- AC:END -->
