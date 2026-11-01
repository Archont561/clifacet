---
id: CLI-28
title: Declarative field attribute for commands
status: To Do
assignee: []
created_date: '2026-11-01 21:18'
labels: []
milestone: m-5
dependencies: []
type: task
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One `#[clifacet(...)]` per field, carrying grammar, interaction and normalization together, so an argument is declared once instead of four times (flag, input field, prompt question, form field list). Design and rationale: decision-13.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `#[clifacet::command]` forwards grammar keys into an `#[arg(...)]` on the same field, and a raw `#[arg(...)]` still passes through untouched.
- [ ] #2 The macro generates `TryFrom<Cli> for the declared input` from the field declarations: copy by name when no rule applies, `trim`/`non_empty`/`secret` when one is declared.
- [ ] #3 `ask`, `fill` and `confirm` generate a collection method that asks only for what no flag supplied, through the same `Session` questions as today.
- [ ] #4 The TUI form's field list is generated from the same declarations rather than hand-written.
- [ ] #5 An unknown key, a `confirm` on a non-boolean, and an `ask` whose parser is not a named path each fail to compile, with a trybuild case and a documented key table.
- [ ] #6 The reference application uses the new syntax, keeps its `ExampleInput`, and every existing parity test passes unchanged.
<!-- AC:END -->
