---
id: decision-5
title: Repository toolchain and ownership boundaries
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer wants the established Archont561 tooling, including Pixi, Alint, Python scripts and pixi-sandbox.

## Decision
Pixi owns the reproducible environment and canonical task surface. Alint owns declarative structure and workflow policy; Python scripts own project-specific semantics. pixi-sandbox owns its sandbox publish/relock generation and developer transport. This repository's own `release.yml` owns the release sequence and is maintained here rather than generated. Cargo owns Rust packages and compilation.

## Consequences
Do not duplicate policy parsing or hand-edit generated workflows. Verify version and capability compatibility before adoption. Cargo and Pixi locks are committed; no unsupported generated workflow is represented as working.

## Implementation note

2026-10-30: one narrow exception to the Pixi-owned toolchain. The `msrv` CI job installs the declared `rust-version` with `rustup` and runs `scripts/prove_msrv.py`, because a Pixi environment pins exactly one compiler and proving an older floor needs a second one. The exception is CI-only: no local task depends on rustup, and `pixi run prove-msrv` uses the pinned compiler unless a toolchain is passed explicitly.
