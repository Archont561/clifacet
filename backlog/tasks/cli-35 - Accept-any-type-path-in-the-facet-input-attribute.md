---
id: CLI-35
title: Accept any type path in the facet input attribute
status: Done
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 09:05'
labels: []
milestone: m-7
dependencies: []
type: task
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The facet input attribute only forwards a token tree that happens to name a struct path; a type with generics, a qualified path or an alias does not survive the round trip. The macro should accept a syn type and either use it or reject it with a message naming the limitation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A qualified path (`crate::input::MyInput`), a generic type (`MyInput<'a>`), and an associated type written as a qualified projection (`<T as Trait>::Input`) are all accepted where the input attribute names a type.
- [x] #2 A form that cannot be supported is rejected with the error span on the offending tokens, not on the whole attribute.
- [x] #3 Each accepted form has a passing test and each rejected one has a trybuild case with the exact message asserted.
- [x] #4 The documented attribute keys describe what the input value may be, and the documentation snippet compiles.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-04: implemented test-first. Baseline before the change: a qualified path and an alias already worked. `Greeting<String>` failed with 'comparison operators cannot be chained' and `Greeting<'a>` with 'expected `:`', because the value was parsed as an expression. `<Marker as Shape>::Input` failed with 'ambiguous associated type', because the macro kept only `.path` and silently dropped the `<Marker as Shape>` qualifier. `Self::Input` failed in rustc with an overflow plus '`Self` is not valid in the self type of an impl'.
- **Design (clean code).** `input` is now read as a `syn::Type` by a small entry parser that both `#[facet(..)]` and `#[clifacet::command(..)]` share, so the two accept exactly the same values (`syn::Meta` can't, because its value is an expression). `bridge` and `check` keep their whole `ExprPath`, so a `<T as Trait>::f` keeps its qualifier too. The generated bridge builds `Self { .. }` rather than the type as written. The command struct's generics flow into the `Facet`, `TryFrom` and `Collect` impls, which is what makes `Greeting<'a>` meaningful on a `struct Cli<'a>`; clap's derive already accepts that. One new module, `clifacet-macros/src/input_type.rs`, walks the type with `syn::visit`. That needs syn's `visit` feature, a feature flag only, so `Cargo.lock` is unchanged.
- **Rejected at the offending tokens**, all in one combined error: `Self`; a lifetime the command struct doesn't declare (`'static` and `for<..>`-bound ones are fine); `'_`; `_`; `impl Trait`; without `bridge`, a type that isn't a struct path (e.g. a tuple); and a value that isn't a type.
- **Evidence.**
  - Red first: 13 new unit tests in `clifacet-macros/src/tests.rs` all failed for their target reason before the change; 53 pass now.
  - `crates/clifacet/tests/input_types.rs` runs each accepted form end to end through `Facet::validate`: qualified path, alias, projection, generic with field rules, and lifetime-generic.
  - Seven trybuild cases (`tests/ui/input_*.rs`) have goldens from `TRYBUILD=overwrite`, each showing a single error spanned on the tokens at fault.
  - Scenario 'An input named through a trait the commands share is built from the declarations' in `examples/simple/tests/features/facet_bridge.feature`.
  - `docs/derive.md` gains a 'What `input` may name' section with a compiled snippet, linked from `docs/command-core.md` and the derive's rustdoc.
  - `test-local` is green; coverage is 98.04%.
- **AC#1 narrowed, not checked; maintainer call.** The qualified path and the generic type are accepted. `Self::Input` cannot be: in `impl Facet for Cli`, `Self::Input` *is* `<Cli as Facet>::Input`, the item being defined, so `type Input = Self::Input` is self-referential (rustc reports overflow), and `Self` is not allowed in the `impl TryFrom<Cli> for …` header at all. It is now rejected at the `Self` token, per AC#2. The associated-type form that can work is a qualified projection, `<Marker as Shape>::Input`, which is accepted and tested. Proposed rewording: '…and an associated type written as a qualified projection (`<T as Trait>::Input`)'. The task is In Progress until that is agreed.

- 2026-11-06: **AC#1 reworded with maintainer approval, then checked.** It used to read '…and an associated type (`Self::Input`) are all accepted…'. `Self::Input` cannot be supported: in `impl Facet for Cli` it is `<Cli as Facet>::Input`, the item being defined, and `Self` is not allowed in the generated `impl TryFrom<Cli> for …` header. It is rejected at the `Self` token, per AC#2 and `tests/ui/input_names_self.rs`. The associated-type form that works, a qualified projection `<T as Trait>::Input`, is what AC#1 now names, and it is accepted and tested (`tests/input_types.rs`, the facet_bridge scenario, the docs snippet). Merged in PR #47 (`ca0865de`). All four ACs are checked; closed with maintainer approval.
<!-- SECTION:NOTES:END -->
