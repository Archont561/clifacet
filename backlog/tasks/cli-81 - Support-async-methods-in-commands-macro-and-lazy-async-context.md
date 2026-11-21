---
id: CLI-81
title: Support async methods in commands macro and lazy async context
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-79
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enhance procedural macros `#[clifacet::commands]` and `#[clifacet::app]` to detect `async fn` methods and automatically generate `AsyncHandler` implementations (decision-26). Support mixed trees containing both synchronous and asynchronous command leaves. Support lazy async context construction (`#[clifacet(context = build)]` with `async fn`) and async lifecycle hooks (`before_run`, `after_success`, `after_failure`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #[clifacet::commands] detects async fn methods and synthesizes AsyncHandler implementations
- [x] #2 Command trees cleanly mix synchronous and asynchronous handler leaves
- [x] #3 Lazy async context construction (FromCli and factory functions) builds only when a command runs
- [x] #4 Async lifecycle hooks (before_run, after_success, after_failure) execute at appropriate stages
<!-- AC:END -->
