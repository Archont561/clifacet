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

## Implementation note

2026-10-30: `Cargo.toml` already declares `rust-version = "1.88"`, which contradicts the consequences above — no CI job has ever compiled at 1.88. The declared floor is therefore unverified, and this decision stays `proposed` until a matrix job proves it (CLI-16). Either add that job or drop the key (CLI-1 AC#3).

## Measurement

2026-10-30, scanning the `rust-version` field of all 284 crates in the vendored dependency graph: no crate requires more than 1.88, and 28 require exactly 1.88 (`ratatui` 0.30.2 and its sub-crates, `time` 0.3.55, `darling` 0.24.1, `trybuild`, `proptest`, `sha2`, `textwrap`, `rstest-bdd` 0.6.0 and its sub-crates, six `icu_*` crates). Twenty-nine require more than 1.85, so 1.85 is already unreachable. The declared 1.88 is therefore the tight floor implied by the locked graph, not an untested guess — but it stays `proposed` until the `msrv` CI job is green, because a declared floor is only proven by compiling with that compiler.
