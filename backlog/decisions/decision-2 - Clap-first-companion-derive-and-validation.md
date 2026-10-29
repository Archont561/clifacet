---
id: decision-2
title: Clap-first companion derive and validation
date: '2026-10-29 15:28'
status: proposed
---
## Context
Clifacet needs to complement existing clap applications without creating a second grammar or exposing private clap parser internals.

## Decision
Propose `#[derive(clap::Args, clifacet::Facet)]` with UI-neutral draft metadata and explicit registration. Clap remains the grammar authority. Validate complete drafts by parsing through the original clap `Command`, with a typed bridge where needed. Preserve supported IDs, aliases, groups, value parsers and environment semantics. Define and report unsupported or non-round-trippable cases rather than promising universal conversion.

## Consequences
Implement and compile a vertical slice before stabilizing the macro/API. Secret-bearing argv reconstruction and arbitrary custom parser or argument combinations need explicit safety and compatibility tests. The exact public API remains proposed.
