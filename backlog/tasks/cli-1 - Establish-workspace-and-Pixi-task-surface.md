---
id: CLI-1
title: Establish workspace and Pixi task surface
status: In Progress
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
- [ ] #3 Rust 2024 and lint policy are centralized; the proposed MSRV is not claimed without proof.
<!-- AC:END -->

## Notes

- 2026-10-29: #1 and #2 verified. A fresh clone builds both crates, and CI's `pixi install --frozen` plus the full quality matrix pass on `3930c68`.
- #3 is open: `Cargo.toml` declares `rust-version = "1.88"`, but no CI job compiles at 1.88, so the floor is claimed without proof. The edition 2024 and lint policy halves are done (`[workspace.package]`, `[workspace.lints]`). Either drop `rust-version` until a matrix exists, or let CLI-16 prove it.
