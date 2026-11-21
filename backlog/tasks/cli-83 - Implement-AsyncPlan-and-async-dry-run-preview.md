---
id: CLI-83
title: Implement AsyncPlan and async dry-run preview
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-79
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement `AsyncPlan<'a, T>` and the `AsyncPlanned` / `AsyncPlannedWith` traits (decision-26). Allow `plan()` itself and the associated work future to be asynchronous while preserving the single-execution-path guarantee of `--dry-run`. Ensure planning operations remain strictly non-mutating even when performing async read-only inspections.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AsyncPlan<'a, T> represents asynchronous work steps and future execution
- [x] #2 AsyncPlanned and AsyncPlannedWith traits allow async preview generation
- [x] #3 --dry-run displays steps without polling the work future
- [x] #4 Parity harness validates that preview descriptions accurately mirror executed effects
<!-- AC:END -->
