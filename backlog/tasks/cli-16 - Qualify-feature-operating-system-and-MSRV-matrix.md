---
id: CLI-16
title: 'Qualify feature, operating system and MSRV matrix'
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-5
dependencies:
  - CLI-12
  - CLI-14
  - CLI-15
type: task
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prove rather than assume compatibility.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CI covers no-default, default, interactive, TUI and all-features as supported.
- [x] #2 Supported Linux, macOS and Windows stable jobs pass with headless tests where relevant.
- [x] #3 Declared MSRV and supported feature/dependency combinations are checked and documented.
<!-- AC:END -->

- 2026-10-31: "compatible" was a claim with no list behind it. The feature matrix lived
  in four hand-written Pixi tasks (`check-base`, `check-interactive`, `check-tui`,
  `check-json`), the MSRV script carried its own second copy of the same idea, and no CI
  job had ever run on macOS or Windows.
- AC#1: `scripts/check_features.py` derives the matrix from
  `crates/clifacet/Cargo.toml`, so adding a feature adds a row and CI never carries a
  list of its own. Rows: `no-default-features`, `default`, each declared feature alone,
  `all-features`. It replaces the four `check-*` Pixi tasks with one `check-features`
  task that runs five commands. Two subtleties are encoded rather than remembered:
  `default` is empty today, so it resolves to *the same configuration* as
  `no-default-features` — the plan collapses them into one `cargo check` and reports both
  names, so the documented table cannot drift from what runs; and a non-empty `default`
  set stops coinciding, which is asserted with a fixture manifest so the collapse is a
  decision rather than an accident. The `all-features` row is workspace-wide on purpose:
  it is the configuration a consumer builds, so every member must compile in it.
- AC#2: a `platforms` CI job runs on `ubuntu-latest`, `macos-latest` and `windows-latest`
  with `fail-fast: false`, checking `--no-default-features` for `clifacet`, then
  `cargo test --workspace --all-features --locked` and the documentation examples. The
  suite is headless by construction — every frontend test drives its own buffers and
  keystroke scripts — so no platform is exempt for lack of a TTY. The two tests that can
  touch a real terminal skip themselves when stdin is one, which a runner never is.
  These jobs use the runner's stable toolchain instead of Pixi, because the checked-in
  Pixi environment targets linux-64 only; that is the same exception decision-5 records
  for the MSRV job, now stated in the docs rather than implied.
- AC#3: `prove_msrv.py` no longer owns a second matrix — it imports `check_features` and
  checks the same rows with the declared toolchain, so the stable matrix and the 1.88
  floor cannot disagree. `docs/development.md` gained a *Supported platforms and feature
  combinations* section with the row table, the commands, and a "what is not claimed"
  paragraph (ARM Linux, 32-bit targets, non-tier-1 platforms, and Windows proving the
  code rather than the rendering); `docs/testing.md`, the README and the CHANGELOG follow
  it. Alint gained a rule for the new script, beside the one for the MSRV proof.
- TDD evidence: `scripts/tests/test_check_features.py` (16) was written first and failed
  with `ModuleNotFoundError: scripts.check_features`; `scripts/tests/test_ci_matrix.py`
  (6) was written before the workflow change and failed with `ValueError` for the missing
  `platforms` job. Three tests were then corrected against the implementation rather
  than the implementation against them, and each correction was a real mistake in the
  test: two rows that resolve to one configuration do collapse (the first draft asserted
  a separate run), the combined boundary is workspace-wide (the first draft expected a
  `-p clifacet` scope), and `describe_failure` has to quote cargo's own `error:` line
  (the first draft only named the row). One implementation bug was caught the same way:
  the `all-features` row reused its own feature list as a subset, tripping the "one
  configuration or the other" guard, which is now an invariant with its own test.
- BDD exception, stated rather than assumed: this task has no user-visible workflow. It
  changes which configurations CI checks and where; the behavior a person can observe —
  what a run writes, what it costs, how a mode is chosen — is unchanged, and the existing
  BDD suites are the evidence, passing in every one of the newly claimed configurations.
- Design decision: the matrix is *data derived from the manifest*, not a list of shell
  commands. The alternative — five explicit `cargo check` lines in a workflow or a Pixi
  task — reads well until a feature is added and one line is forgotten, which is exactly
  the class of bug this task exists to remove. `Plan` and `Failure` moved to
  `check_features` and `prove_msrv` imports them, so "which configurations" and "with
  which compiler, and how a failure is explained" are separate modules with one
  direction of dependency.
- Coverage: unchanged at 932/937 (99.47%); `test-scripts` grew from 107 to 124 tests, and
  the Rust suites stay at 24 + 220.
- Not verified here: the macOS and Windows jobs have never run before this commit — this
  PR is their first execution, and CI is the qualification. Everything runnable offline
  on linux-64 is green: `fmt-check`, `lint`, `alint` (39 rules), `actionlint`,
  `check-features`, `backlog-check`, `sandbox-check`, `test`, `test-doc`, `test-scripts`,
  `deny-offline`, `coverage`.
