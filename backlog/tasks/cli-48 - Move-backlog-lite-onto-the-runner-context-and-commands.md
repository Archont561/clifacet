---
id: CLI-48
title: 'Move backlog-lite onto the runner, context and commands!'
status: To Do
assignee: []
created_date: '2026-11-08 01:59'
labels: []
milestone: m-12
dependencies:
  - CLI-46
  - CLI-47
  - CLI-33
type: task
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The proof that decision-19's first sprint pays for itself, in the repository's only tree. Estimated in the review: app.rs (120 lines) goes, main.rs (17 lines) becomes one call, commands/mod.rs (125 lines) becomes about ten, invocation.rs (134 lines) becomes about sixty as a helper module, and the places touched to add a command go from seven to one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 main.rs is one call to the runner, app.rs is gone, and commands/mod.rs is a `clifacet::commands!(..)` registration
- [ ] #2 The program backlog-lite spawns is part of its context, so the CLIFACET_BACKLOG_BIN variable is gone and the tests inject a fake
- [ ] #3 The thread_local! run records in crates/clifacet/tests/command_tree.rs and the library BDD bindings are replaced by a context
- [ ] #4 Every existing example test and scenario passes; bindings change only how they wire the application, never what they assert
- [ ] #5 The notes record the line counts before and after and the number of places touched to add a command
<!-- AC:END -->
