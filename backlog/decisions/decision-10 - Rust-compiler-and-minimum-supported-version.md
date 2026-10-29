---
id: decision-10
title: Rust compiler and minimum supported version
date: '2026-10-29 15:28'
status: proposed
---
## Context
Edition 2024 and a stable Pixi-pinned compiler match the surrounding Rust repositories. A reusable library benefits from an explicit compatibility floor.

## Decision
Use Rust 2024 and a stable developer toolchain pinned by Pixi. Consider Rust 1.88 as the initial MSRV, subject to resolving and compiling the actual complete supported feature and transitive dependency graph. Run a distinct MSRV CI job.

## Consequences
Do not claim 1.88 support until tested. Align `rust-version`, feature policy and CI; decide how updates to transitive dependencies affect the floor. No Rust compiler was available during this planning session.
