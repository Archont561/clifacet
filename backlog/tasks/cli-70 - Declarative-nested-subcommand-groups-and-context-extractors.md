---
id: CLI-70
title: Declarative nested subcommand groups and context extractors
status: To Do
assignee: []
created_date: '2026-11-19 14:00'
labels: []
milestone: m-18
dependencies:
  - CLI-67
  - CLI-68
  - CLI-69
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement declarative nested subcommand groups and context extractors (decision-24). Allow child subcommand groups to be mounted directly as fields on the root struct (`#[clifacet(subcommand)]`) or child groups. Support child group context slicing via `#[clifacet(extract)]` or `#[clifacet(context)]`, automatically synthesizing child constructors from the parent context via `AsRef` or `From`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Subcommand groups can be declared as fields on the root struct using #[clifacet(subcommand)]
- [ ] #2 Child group structs can declare context slices via #[clifacet(extract)], auto-extracted from parent context
- [ ] #3 Multi-level nested commands resolve properly through the runner pipeline and argument parser
- [ ] #4 Integration tests verify multi-level subcommand dispatch, independent command state, and context slicing
<!-- AC:END -->
