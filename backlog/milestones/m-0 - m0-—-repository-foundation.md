---
id: m-0
title: "M0 — Repository foundation"
---

## Description

Create the reproducible repository toolchain and workflow skeleton.

Exit condition: CI-quality gates pass on a fresh clone; sandbox restore/offline path is verified; release integration has a documented verified approach.

## Exit criteria

Recorded 2026-10-30 against `main` at `6ae7116`. Each line cites evidence; none is inferred from configuration.

- [x] **CI-quality gates pass on a fresh clone.** CI run `41077737064` is green on `6ae7116` for the repository and Rust quality job (alint, skills, Backlog, feature matrix, `cargo fmt`, Clippy with `-D warnings`, `cargo deny`, `cargo audit`) and for the tests and coverage job (script tests, `cargo llvm-cov` gate, doctests, Codecov).
- [x] **Sandbox restore and offline path are verified.** Publish run `41077736958` republished `sandbox/developer-linux-64` to `dc6d062`. A cold clone of that branch reported `doctor --verify` — 14,874 blobs, 1,032.9 MiB, 0 failures — and a restore into a fresh clone of `main` matched the manifest. With no network, `pixi run --no-install --offline test-scripts` ran 24 tests and `alint` passed 28 rules.
- [ ] **Release integration has a documented verified approach.** Not started: CLI-4.

## Remaining

- **CLI-4** is the only exit clause with no work at all. M0 cannot close without it.
- **CLI-1 AC#3** keeps M0's toolchain clause honest: `Cargo.toml` declares `rust-version = "1.88"` while no CI job compiles at 1.88, contradicting decision-10. Either add the matrix job or drop the key; CLI-16 is the planned home for the proof.
- **CLI-19** tracks the RUSTSEC-2026-0374 exception and now belongs to m-4 instead of floating unassigned.
