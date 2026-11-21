---
id: CLI-80
title: Implement Executor trait TokioExecutor and async run entrypoints
status: Done
assignee: []
created_date: '2026-11-19 19:00'
labels: []
milestone: m-21
dependencies:
  - CLI-79
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define the pluggable `Executor` trait to drive async commands from synchronous pipelines (decision-26). Provide `TokioExecutor` under the opt-in `tokio` feature. Add `App::run_async` and `App::run_with_async` entry points for applications already executing inside an active runtime. Ensure invoking `App::run` inside an active Tokio runtime returns an actionable `Failure::execution` rather than panicking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Executor trait is defined to execute async commands synchronously through a chosen runtime
- [x] #2 tokio feature provides TokioExecutor supporting current-thread and multi-thread configurations
- [x] #3 App::run_async and App::run_with_async execute natively in existing async runtimes
- [x] #4 Invoking synchronous App::run inside an existing runtime returns Failure::execution with a helpful diagnostic
<!-- AC:END -->
