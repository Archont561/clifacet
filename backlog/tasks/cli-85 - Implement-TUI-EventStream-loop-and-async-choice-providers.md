---
id: CLI-85
title: Implement TUI EventStream loop and async choice providers
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-80
  - CLI-78
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Refactor the TUI runner under `tui` + `async` to run command work as an async task while selecting over crossterm `EventStream` (decision-26). Ensure terminal raw mode is properly cleaned up on hard cancellation or panic. Add async choice providers (`#[clifacet(choices = load)]`) supporting asynchronous lookups sharing contracts with dynamic shell completions (CLI-78).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 TUI loop selects over crossterm EventStream, redrawing while background tasks execute
- [x] #2 Drop guards guarantee terminal restoration to cooked mode even on hard cancellation or panic
- [x] #3 #[clifacet(choices = ...)] supports async callback functions accessing &Cx
- [x] #4 Async choice callbacks integrate cleanly with dynamic shell completion hooks
<!-- AC:END -->
