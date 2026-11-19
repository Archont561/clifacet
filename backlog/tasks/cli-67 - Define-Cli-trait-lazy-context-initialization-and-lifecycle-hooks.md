---
id: CLI-67
title: Define Cli trait, lazy context initialization, and lifecycle hooks
status: Done
assignee: []
created_date: '2026-11-19 14:00'
labels: []
milestone: m-18
dependencies: []
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define the public `clifacet::Cli` and `clifacet::FromCli` traits (decision-24). `Cli` establishes the application lifecycle contract with associated `type Context`, lazy factory `init_context(&self) -> Result<Self::Context, Failure>`, and hook methods (`before_run`, `after_success`, and `after_failure`) with default no-op implementations. Integrate these lifecycle hooks into the `App` runner pipeline, guaranteeing that `init_context` is never invoked during `--help`, `--version`, or argument parsing validation errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 clifacet::Cli trait is defined with type Context, init_context, before_run, after_success, and after_failure
- [x] #2 clifacet::FromCli trait is defined to construct application contexts from parsed CLI root options
- [x] #3 App runner pipeline executes lifecycle hooks in order and skips init_context on --help, --version, or usage errors
- [x] #4 Integration tests verify hook execution order, error propagation, and context cleanup during success and failure
<!-- AC:END -->
