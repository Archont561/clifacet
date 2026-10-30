---
id: CLI-8
title: Add core rstest and property test foundations
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-1
dependencies:
  - CLI-7
type: task
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Set up fixtures and property tests around the agreed command seams.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rstest fixtures and cases cover draft, input and validation behavior.
- [x] #2 Proptest covers deterministic resolution and supported non-secret argument round trips.
- [x] #3 Document actual crate versions and verified compatibility.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `crates/clifacet/tests/validation_matrix.rs` (fixtures plus four case
  tables), `crates/clifacet/tests/properties.rs` (four invariants), and `docs/testing.md` (installed
  versions, proven MSRV scope, coverage rule). No production code changed: this task is test
  foundation, and the seams it exercises were already there.
- AC#1: the tables are separated by layer, not by flag. `draft` cases assert what `clap` collected —
  including `--repeat 9`, which must survive to validation rather than be rejected by the table —
  `input` cases assert the normalized value, and `validation` cases assert the exact user-facing
  message and `FailureKind::Validation`. Fixtures chain argv → draft → input so a failure names the
  layer instead of the plumbing.
- AC#2: round trip is stated over *canonical* input (trimmed, 1..=8), because validation normalizes:
  `--name " ada "` does not and must not round trip unchanged. Normalization itself is a separate
  property ("an accepted name is trimmed and non-blank; a refused one was blank"). Determinism is
  asserted over arbitrary argv, which is also where the exit-code and one-stream-only invariants live.
- AC#3: `docs/testing.md` records the pinned versions with each crate's declared `rust-version`, and
  states the limit plainly: the MSRV job runs `cargo check`, so test-only dependencies are covered by
  their declared floors, not by a 1.88 build. `rstest-bdd` 0.6.0 keeps its `=` pin, and the
  `basic-toml` advisory path stays with CLI-19.
- TDD evidence: the first red was `every_argv_lands_on_a_documented_exit_code`, which failed on
  `["--name", "ada"]`. It found two real defects in the test harness, not in the product: the harness
  did not prepend the program name `main` receives, and the invariant claimed every run writes to the
  data stream, which a usage error correctly does not. The invariant was corrected and kept strict
  (success ⇒ data and no diagnostic; failure ⇒ a diagnostic; never both streams), rather than
  weakened to match the bug. The shrunk seed is committed.
- BDD evidence, and the narrow exception recorded per AGENTS: this slice adds no user-visible
  workflow, so no new `.feature` was written — decision-6 and the bdd skill both say combinatorial and
  infrastructure behavior belongs in tables and properties. The existing
  `tests/features/facet_bridge.feature` continues to cover the public promise it exercises.
- Proptest seeds persist to `crates/clifacet/tests/proptest-regressions/` (configured explicitly rather
  than relying on proptest's fallback file name) and are committed, per decision-6.
- Deliberately out of scope: property tests for the interactive draft lifecycle (CLI-9 and CLI-12 own
  the draft's screen state), and any proptest of the derive's expansion, which the in-process unit
  tests in `clifacet-macros` already cover exhaustively.
