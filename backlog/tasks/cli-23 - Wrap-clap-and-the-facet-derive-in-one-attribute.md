---
id: CLI-23
title: Wrap clap and the Facet derive in one attribute
status: Done
assignee: []
created_date: '2026-11-01 16:00'
labels: []
milestone: m-1
dependencies:
  - CLI-7
type: task
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`#[derive(clap::Parser, clifacet::Facet)]` plus `#[facet(input = …)]` is two attributes
that have to agree, and getting it half right is a confusing compile error. Provide the
pair as one attribute.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `#[clifacet::command(input = …)]` produces the same both-derive shape, with clap's grammar untouched and the caller's own attributes preserved.
- [x] #2 It emits the existing derive rather than a second implementation, and `#[derive(Facet)]` keeps working on its own.
- [x] #3 Each way it can be misused is a compile error that says what to write instead, with golden `trybuild` files.
<!-- AC:END -->

## Notes

- 2026-11-01: **why an attribute and not a third derive.** A derive macro cannot add
  another derive to the item it is attached to — derives only contribute impls — so
  `#[derive(clifacet::Command)]` could not produce clap's parser. An attribute macro
  re-emits the item with both derives in front of it, which is the whole trick. The
  alternative, implementing `clap::Parser` from scratch, would be exactly the second
  grammar decision-2 forbids.
- 2026-11-01: **the one thing it cannot hide.** `clap`'s derive expands to code naming
  `clap::…`, and no attribute can redirect that path: clap 4.6.7's derive has no `crate`
  override (checked in the vendored source). So the calling crate still needs `clap` as a
  direct dependency. `clifacet` re-exports `clap` so the version is the one the macro was
  built against, and both the guide and the rustdoc say this plainly rather than implying
  the dependency is gone.
- 2026-11-01: the name is `command`, in the macro namespace, sitting beside the
  `clifacet::command` module — Rust keeps types and macros in separate namespaces, so
  neither shadows the other. The doctest in `lib.rs` proves it, and a comment there says
  why it is safe.
- 2026-11-01: the item is parsed as `syn::DeriveInput` (struct/enum/union) and re-emitted,
  not as `syn::Item`, which needs syn's `full` feature and is not enabled here. `Data::
  Struct` is checked explicitly, because clap supports subcommand enums but the field-by-
  field bridge has no fields to copy and the error would otherwise arrive later and
  somewhere else.
- 2026-11-01: reusable diagnostics — a second `#[facet(..)]`, a second `Parser`, a second
  `Facet`, a missing `input`, and an enum each fail with a message naming the attribute
  the caller actually wrote (`#[clifacet::command(input = YourInputType)]`), not the one
  the expansion emits.
- Also fixed here: the `Facet` derive's own doc comment advertised a
  `#[facet(validate = path)]` key that the derive has never supported. Corrected to
  `bridge`, which is what it reads. The wrapper documents "the same keys as `#[facet(…)]`",
  so leaving the stale claim in place would have spread it.
- TDD evidence: eleven expansion tests were written first and failed with
  `unresolved import crate::command`; then `syn::Item` failed to compile without the
  `full` feature and the missing `parse::Parser` import surfaced, both fixed by the red
  test. `TRYBUILD=overwrite` generated the two golden `.stderr` files, and
  `tests/ui/command_valid.rs` compiles *and runs* — it parses `--nickname` and asserts the
  bridge produced `Some("ada")`. The guide snippet compiles as a doc test through
  `check-docs` (9 snippets, was 8).
- BDD: narrowed and stated — this is a compile-time ergonomics change with no runtime
  workflow, so a `rstest-bdd` scenario would describe a macro expansion rather than a
  promise to a user. The promise ("one attribute produces the pair, and misuse says what
  to write") is covered by expansion tests plus golden compiler output.
- Clean code: the wrapper holds no second implementation of anything — its output is
  `#[derive(::clap::Parser, ::clifacet::Facet)]` and `#[facet(..)]`, so `clap` and the
  derive stay the single sources of truth. Validation is three small helpers with one
  job each, and the caller's tokens are re-emitted rather than normalised.
