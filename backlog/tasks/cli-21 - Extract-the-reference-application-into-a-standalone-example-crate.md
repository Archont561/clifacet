---
id: CLI-21
title: Extract the reference application into a standalone example crate
status: Done
assignee: []
created_date: '2026-10-31 00:00'
labels: []
milestone: m-5
dependencies:
  - CLI-15
type: task
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move the reference application and the tests that drive it out of the published `clifacet`
library and into a workspace crate of its own, so the example is a consumer of the public API
rather than part of the library it demonstrates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `clifacet` no longer ships the example: no `example` module, no example code in the published crate.
- [x] #2 The reference application lives in `crates/clifacet-example`, depending on `clifacet` only through its public API.
- [x] #3 The tests, BDD scenarios and runnable demos that exercise the application move with it and still pass unchanged in behavior.
- [x] #4 Repository policy, docs and CI follow the new layout: workspace members, Alint rules, `command-core`/`development`/`testing` pages, README and changelog.
<!-- AC:END -->

## Notes

- 2026-10-31: the reference application became `crates/clifacet-example` (package
  `clifacet-example`, `publish = false`). `crates/clifacet/src/example.rs` is gone, so
  `clifacet::example` is no longer published; the library's `lib.rs`, `command.rs`,
  `failure.rs` and `tui/mod.rs` now name the crate instead of linking to a module that
  shipped inside the library.
- AC#1: `pub mod example` is removed and `Cargo.toml` no longer declares the two
  `[[example]]` targets. Nothing in `crates/clifacet/src` refers to the example; the
  three doc sentences that linked to `crate::example` point at the crate by name.
- AC#2: the crate depends on `clap` (the derive expands to `::clap::…` paths) and on
  `clifacet = { path = "../clifacet", features = ["tui"] }` — nothing private, no
  features of its own. Every `crate::` path in the moved code became `clifacet::`; the
  one `pub(crate)` helper (`terminal_frontend`) is now public because a consumer of the
  application reaches it. The two runnable demos moved with it
  (`examples/parity_report.rs`, `examples/tui_custom_screen.rs`) and the binary became
  `src/main.rs`, so `cargo run -p clifacet-example -- --tui` needs no `--example` flag.
- AC#3: 17 test files, all eight `.feature` files and the proptest seeds moved with the
  application; only the tests that never touched the example stayed
  (`compile_fail`, `harness`, `interaction_policy`, `prompting`, `tui_form`, `tui_keys`,
  `version`). The suite is the evidence: `test-src` 24 and `test-integration` 220 pass,
  against 29 + 218 before. The five unit tests dropped from `src/tests.rs` were
  redundant — `example_command.rs` already asserts the greeting count, the inclusive
  upper bound and the execution-failure kind through better seams, and `outcomes.rs`
  already asserts the exit mapping — and the policy-ceiling assertion stayed in the
  library. Two new tests replace the ones that could only exist inside the library:
  `the_process_frontend_is_constructible_without_touching_a_terminal` and
  `the_application_offers_every_mode_the_library_has`.
- AC#4: workspace membership, an Alint rule for the crate's manifest, the
  `require-example-command` path, `command-core.md`, `development.md`, `testing.md`,
  README and CHANGELOG all follow the new layout; `cargo fmt --all`, `lint`, `alint`,
  `deny-offline`, `audit-offline`, `test`, `test-doc`, `test-scripts`, `actionlint`,
  `sandbox-check`, `skills-check`, `backlog-check` and `coverage` are green.
- TDD/BDD exception, stated rather than assumed: this is a move with no new behavior, so
  there is no failing test to write first and no new user-visible workflow to describe.
  The BDD suites moved unchanged and still pass (the `.feature` files keep their
  relative paths under `tests/features/`), which is the stronger evidence: the same
  scenarios describe the same promise from a consumer's crate.
- Design decisions: the example enables the frontends it demonstrates unconditionally
  (`features = ["tui"]` on its `clifacet` dependency) instead of mirroring the library's
  feature list. A demo crate with its own switches would have to test each combination
  to stay honest, and the library already owns the "a mode nothing can draw is not a
  mode" rule in `policy::Refusal`. `POLICY` is therefore `Policy::up_to(Mode::Tui)` and
  the `#[cfg(feature = "tui")]` gates in the ported code are gone. The tests import the
  crate as `use clifacet_example::{self as example, …}`, so the prose in every scenario
  still reads "the example's …" while the path names the real crate.
- Dependency policy: making `clifacet/tui` a normal dependency brings `ratatui` into
  cargo-deny's default graph for the first time, which surfaced a real gap — `foldhash`
  (through `hashbrown`) is Zlib. It is now a scoped license exception in `deny.toml`
  beside the existing `unicode-ident` one, rather than a broadening of the global
  allow-list. `allow-wildcard-paths` is enabled for the same reason the example is
  versionless: a `publish = false` workspace crate depends on its sibling by path.
- Coverage: the gate now reports 932/937 (99.47%) because it counts the published
  crates only; the example's own lines are exercised by its suite but are not part of
  the published-coverage contract, which is now stated in `development.md`.
- Deliberately out of scope: publishing the example crate (it stays `publish = false`,
  so the release workflow still ships exactly two crates) and the `interactive` feature
  (nothing in the library uses `inquire` yet, so the example does not enable it).

