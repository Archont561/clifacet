---
id: CLI-1
title: Establish workspace and Pixi task surface
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies: []
type: task
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create two Cargo crates, workspace metadata and a reproducible Pixi environment without claiming public API completion.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Fresh clone resolves the intended Cargo and Pixi toolchains and lockfiles; both crates build.
- [x] #2 Canonical Pixi formatting, lint, test and build tasks are documented and usable.
- [x] #3 Rust 2024 and lint policy are centralized; the proposed MSRV is not claimed without proof.
<!-- AC:END -->

## Notes

- 2026-10-29: #1 and #2 verified. A fresh clone builds both crates, and CI's `pixi install --frozen` plus the full quality matrix pass on `3930c68`.
- #3 is open: `Cargo.toml` declares `rust-version = "1.88"`, but no CI job compiles at 1.88, so the floor is claimed without proof. The edition 2024 and lint policy halves are done (`[workspace.package]`, `[workspace.lints]`). Either drop `rust-version` until a matrix exists, or let CLI-16 prove it.
- 2026-10-30: AC#1 re-verified on `main` at `6ae7116` — CI run `41077737064` is green, including the feature matrix and both crate builds. AC#3 remains the only open criterion; decision-10 now carries an implementation note recording the contradiction.
- 2026-10-30: AC#3's proof now exists as work in progress. `scripts/prove_msrv.py` reads `[workspace.package] rust-version` and checks the empty feature set, each optional feature and all features with `--locked --all-targets`; the new `msrv` CI job installs that exact toolchain with rustup and runs it. A scan of all 284 vendored crates shows nothing requires more than 1.88 and 28 require exactly 1.88 (decision-10). AC#3 stays open until that job is green on `main`.
- 2026-10-30: AC#3 proven. The `msrv` job (run 41087537994) installed rust 1.88 and compiled the empty feature set, `interactive`, `json`, `tui` and all features with `--locked --all-targets`, all green. The claim now rests on a check rather than on a manifest field.
