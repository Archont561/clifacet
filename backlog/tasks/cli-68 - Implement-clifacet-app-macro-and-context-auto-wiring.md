---
id: CLI-68
title: Implement clifacet app macro and context auto-wiring
status: Done
assignee: []
created_date: '2026-11-19 14:00'
labels: []
milestone: m-18
dependencies:
  - CLI-67
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the `#[clifacet::app]` procedural macro attribute (decision-24). The macro inspects a root controller struct, parses CLI flags, options, and defaults, mounts subcommand groups via `#[clifacet(subcommand)]`, and identifies context fields marked with `#[clifacet(context)]`. Generates the underlying argument parser and synthesizes the `clifacet::Cli` trait implementation, auto-wiring `init_context` through `FromCli`, `Default`, or a custom factory.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #[clifacet::app] parses struct fields into CLI options, subcommands, and context markers
- [x] #2 Macro synthesizes the underlying argument parser and derives clifacet::Cli with auto-wired init_context
- [x] #3 Macro emits descriptive compile errors when context annotations are invalid or missing required trait bounds
- [x] #4 Trybuild tests verify expansion correctness and compiler diagnostic quality for invalid configurations
<!-- AC:END -->
