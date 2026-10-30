---
id: CLI-7
title: Implement companion derive and clap validation slice
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-1
dependencies:
  - CLI-6
type: task
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prove the proposed clap-first derive against a representative existing-style clap command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A derived example resolves non-interactively and final validation uses its original clap Command or a documented typed bridge.
- [x] #2 Aliases, defaults and a custom parser representative case are tested; unsupported cases fail explicitly.
- [x] #3 trybuild covers valid and invalid derives; no private clap APIs are used.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `clifacet::Facet` — a trait with a `command()` default that *is*
  `<Self as CommandFactory>::command()`, plus `#[derive(Facet)]` in `clifacet-macros` that implements
  `validate`. The example application now parses an alias (`--nickname`, `-n`), a clap default
  (`--repeat`), and a custom `value_parser` (`--level`), and reaches typed input only through
  `Facet::validate` — `example::run` uses the same call the tests do.
- AC#1, "its original clap Command or a documented typed bridge": both. The derive exposes clap's own
  command (`tests/facet_bridge.rs` compares `render_long_help()`), and the final validation is
  `#[facet(bridge = build_input)]`, documented in `docs/command-core.md` and in the trait's rustdoc.
  No argv is rebuilt and no second grammar exists, so a private clap API has nothing to be needed for.
- AC#2: alias, default and custom parser each have a test; the unsupported shapes are compile errors with
  golden diagnostics — a derive without `#[facet(input = …)]`, an unknown key, an enum, and an input
  missing a field the mirror copies. `--level chatty` exits 2 with `clap`'s own message, measured.
- AC#3: `crates/clifacet/tests/compile_fail.rs` runs two passing derives (mirror and bridge) and four
  failing ones under `tests/ui/`. Public API only: `Command`, `CommandFactory`, `Arg`, `ArgMatches`.
- TDD evidence (AGENTS "New feature work"): the first failing test was `tests/facet_bridge.rs`; it failed
  with seven missing-seam errors (`no method named validate`, no `Facet`, no `Level`, no `level` field)
  before any production code existed.
- BDD evidence: `tests/features/facet_bridge.feature` with `tests/facet_bdd.rs` — one scenario walked by
  `rstest-bdd` steps, asserting the alias is clap's and that the run needs no terminal.
- Clean-code decisions: the derive has exactly two input strategies (field mirror, declared bridge) and
  documents why both exist; the derive rejects shapes it cannot bridge at compile time instead of
  ignoring them; the level's rules live with `Level` (`FromStr`), not in the macro or the application.
- Narrow exception recorded: none requested. The CLI-7 changes are user-visible (a new flag, a new
  output shape for `--level verbose`), so the full TDD/BDD route applied.
- Deliberately out of scope: canonical argv replay, and any clap construct that cannot be re-validated
  (`docs/command-core.md` "Known unsupported behavior"). rstest/proptest foundations stay with CLI-8.
