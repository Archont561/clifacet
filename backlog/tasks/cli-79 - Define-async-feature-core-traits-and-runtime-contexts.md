---
id: CLI-79
title: Define async feature core traits and runtime contexts
status: To Do
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies: []
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define the `async` Cargo feature without any third-party runtime dependencies (decision-26). Introduce the core `AsyncHandler`, `AsyncCommand`, and `AsyncPlanned` traits using native AFIT and RPITIT (`std::future::Future`). Introduce `RunCx` carrying cancellation (`Cancel`) and progress streaming (`ProgressSink`), plus `LocalAsyncHandler` for single-threaded or `?Send` environments.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 async Cargo feature is defined with zero runtime dependencies (std futures only)
- [ ] #2 AsyncHandler, AsyncCommand, AsyncPlanned, and LocalAsyncHandler traits are implemented
- [ ] #3 RunCx, Cancel token, and ProgressSink trait are defined and passed to async handlers
- [ ] #4 Unit tests and contract tests verify trait implementations and test harness helpers (RunCx::for_test)
<!-- AC:END -->
