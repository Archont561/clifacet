---
id: CLI-28
title: Declarative field attribute for commands
status: In Progress
assignee: []
created_date: '2026-11-01 21:18'
updated_date: '2026-11-03 07:49'
labels: []
milestone: m-7
dependencies: []
type: task
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One `#[clifacet(...)]` per field, carrying grammar, interaction and normalization together, so an argument is declared once instead of four times (flag, input field, prompt question, form field list). Design and rationale: decision-13.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #1 `#[clifacet::command]` forwards grammar keys into an `#[arg(...)]` on the same field, and a raw `#[arg(...)]` still passes through untouched.
- [ ] #2 #2 The interaction keys (`ask`, `fill`, `confirm`) and normalization keys (`trim`, `non_empty`, `secret`) are parsed and consumed by the generated collection method and `TryFrom` bridge; naming one today is a compile error, not a silent no-op.
- [ ] #3 #3 The macro generates `TryFrom<Cli> for the declared input` from the field declarations: copy by name when no rule applies, `trim`/`non_empty`/`secret` when one is declared.
- [ ] #4 #4 An unknown key, a `confirm` on a non-boolean, and an `ask` whose parser is not a named path each fail to compile, with a trybuild case and a documented key table.
- [ ] #5 #5 The TUI form's field list is generated from the same declarations rather than hand-written.
- [ ] #6 #6 The reference application declares its flags with the new syntax, keeps its `ExampleInput`, and every existing parity test passes unchanged.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-03: AC#1 landed; AC#2 and AC#3 were rewritten on the way, and this note says why rather than leaving the renumbering to be inferred. The old AC#2 required the macro to *generate* the input bridge, which it already does today through `#[facet(input = …)]` and `#[derive(Facet)]` — the derive copies fields by name whether the field used `#[arg(..)]` or `#[clifacet(..)]`, because the attribute is consumed into an `#[arg(..)]` before clap's derive and `Facet` see the field. Writing a second bridge generator would have been the duplicate this crate exists to avoid. The AC now says what is actually owed: the interaction and normalization keys have to be *consumed* by the generated collection and bridge instead of being forwarded; today they are rejected with 'not a key this attribute forwards'.
- Grammar keys are forwarded as tokens, never reinterpreted: `short = 'n'`, `value_parser = clap::value_parser!(u8).range(1..=8)` and `range = 1..=8` all reach clap exactly as written, which keeps decision-2 intact. The key table is split three ways — 17 value keys, 5 flag keys, and `long`/`short` accepted either bare or with a value, because clap accepts both and a single category would have made `#[clifacet(long)]` fail on the most common spelling.
- Two rules, both tested: an unknown key is a compile error whose message names the supported keys and points at the key rather than the whole attribute, and a key declared twice is an error rather than last-one-wins. Both have golden trybuild files, including the full key table — the guide and the goldens are edited together on purpose, so the surface cannot drift from its documentation.
- The escape hatch is tested as a composition, not just as passthrough: `tests/ui/field_attribute_valid.rs` declares one field with `#[clifacet(..)]` and another with a raw `#[arg(..)]`, and asserts clap's rendered help shows both.
- BDD: `examples/simple/tests/features/facet_bridge.feature` gained 'A field declared once carries its own grammar' with four steps, driven through public seams only — the help is clap's `render_help()`. The declared command lives in the example crate's tests rather than in `examples/simple/src` because AC#6 moves the reference application over once the interaction half exists; moving it now would have changed the example twice.
- Docs: `docs/derive.md` gained a 'Declaring one field once' section with the full three-way key table and the escape-hatch rule; its snippet is compiled by `check-docs` (13 snippets, 10 pages). `docs/development.md` states the new cfg-site guard and the single `json` exception.
- Evidence: `trybuild` 3 cases pass; workspace 302 Rust tests; 315 Python tests; `check-features` 5 configurations compile; clippy `-D warnings` clean; `cargo fmt --check` clean; `check-docs` clean.
- Still open on this card: AC#2 to AC#6 — the interaction/normalization keys, the generated collection, the form field list, and the reference application migration.
<!-- SECTION:NOTES:END -->
