---
id: decision-5
title: Repository toolchain and ownership boundaries
date: '2026-10-29 15:28'
status: accepted
---
## Context
The maintainer wants the established Archont561 tooling, including Pixi, Alint, Python scripts, pixi-sandbox and pixi-release.

## Decision
Pixi owns the reproducible environment and canonical task surface. Alint owns declarative structure and workflow policy; Python scripts own project-specific semantics. pixi-sandbox owns its sandbox publish/relock generation and developer transport. pixi-release owns its release policy and supported generated workflow surfaces. Cargo owns Rust packages and compilation.

## Consequences
Do not duplicate policy parsing or hand-edit generated workflows. Verify version and capability compatibility before adoption. Cargo and Pixi locks are committed; no unsupported generated workflow is represented as working.
