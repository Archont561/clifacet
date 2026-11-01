---
id: decision-2
title: Clap-first companion derive and validation
date: '2026-10-29 15:28'
status: accepted
---
## Context
Clifacet needs to complement existing clap applications without creating a second grammar or exposing private clap parser internals.

## Decision
Use `#[derive(clap::Parser, clifacet::Facet)]` with UI-neutral draft metadata and explicit registration, or `#[clifacet::command(input = …)]`, which emits that derive rather than a second implementation. Clap remains the grammar authority. Validate complete drafts by parsing through the original clap `Command`, with a typed bridge where needed. Preserve supported IDs, aliases, groups, value parsers and environment semantics. Define and report unsupported or non-round-trippable cases rather than promising universal conversion.

## Consequences
Implement and compile a vertical slice before stabilizing the macro/API. Secret-bearing argv reconstruction and arbitrary custom parser or argument combinations need explicit safety and compatibility tests. The exact public API stays small: one derive, one attribute, one trait, and a documented list of shapes it refuses.

## Acceptance note

2026-11-01: accepted, with `0.1.0` shipped against it. The vertical slice the Consequences
required came first and compiled as CLI-7 (the `Facet` derive plus validation by parsing
through the original clap `Command`), and CLI-23 tightened it so one attribute emits both
derives instead of two that have to agree; `docs/derive.md` states what it does not guess.
Three clauses are recorded here because they were met by narrowing rather than by
promising everything:

- **Unsupported shapes fail at compile time, not at runtime.** Each of them — a derive
  without `#[facet(input = …)]`, an unknown attribute key, an enum, and an input missing a
  field the mirror would copy — has a `trybuild` case in `crates/clifacet/tests/ui/`, so a
  caller learns before packaging a binary rather than when a user reaches the prompt.
- **Secret-bearing argv is protected by type, not by reconstruction.** The framework never
  rebuilds an argv — parsing stays clap's — so `secret::Secret` is what keeps a credential
  out of everything the framework prints (CLI-14), and there is no recomposition path to
  test for leaks.
- **Alias and default semantics are preserved by delegating**, not re-implemented: the
  derive reports `clap`'s own `Command`, so `alias` and `default_value_t` need no code path
  of their own, and `examples/simple/tests/facet_bridge.rs` asserts the parity rather than
  trusting it.

The last line of Consequences no longer applies: `0.1.0` makes the API semver-committed,
and accepting this decision is precisely the statement that the slice was enough to build
against.
