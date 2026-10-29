---
id: CLI-1
title: Establish workspace and Pixi task surface
status: To Do
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
- [ ] #1 Fresh clone resolves the intended Cargo and Pixi toolchains and lockfiles; both crates build.
- [ ] #2 Canonical Pixi formatting, lint, test and build tasks are documented and usable.
- [ ] #3 Rust 2024 and lint policy are centralized; the proposed MSRV is not claimed without proof.
<!-- AC:END -->
