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
- [x] **Release integration has a documented and verified approach.** `.github/workflows/release.yml` implements trusted publishing (CLI-4); dry-run release `41085434034` is green on `afcf91b`, covering setup-pixi, version resolution, the consistency guard and macro-crate packaging. Caveat kept honest: the live publish path (OIDC exchange, `cargo publish`, index polling) is unexercised until a real release, which CLI-18 owns, and the `crates` environment currently has no protection rules.

## Status

**Complete as of 2026-10-30.** All three exit clauses are ticked, every m-0 task is Done (CLI-1, CLI-2, CLI-3, CLI-4, CLI-5), and CLI-19 moved to m-4. The MSRV proof landed with it: `msrv` job run `41087537994` compiled the dependency graph at rust 1.88. CLI-16 still owns the fuller operating-system and feature qualification, which is m-5 work.

## Remaining

- **CLI-4** is done as of 2026-10-30; the only outstanding risk it leaves is the environment gate above.
- **CLI-19** tracks the RUSTSEC-2026-0374 exception and now belongs to m-4 instead of floating unassigned.
