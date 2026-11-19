---
id: CLI-71
title: Retire build api and directory scanning
status: Done
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-18
dependencies:
  - CLI-67
  - CLI-68
  - CLI-69
  - CLI-70
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Retire the `clifacet::build` module, `clifacet::build::commands()`, and the `build` Cargo feature (decision-25). Declarative controller structs and `impl` blocks replace `build.rs` filesystem directory scanning and the `CLIFACET_COMMANDS` environment variable protocol. Clean up legacy build script discovery tests and update documentation guides.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 clifacet::build module and build Cargo feature are deprecated and removed
- [x] #2 CLIFACET_COMMANDS environment variable protocol is eliminated from macro crate
- [x] #3 Documentation and example crates are updated to demonstrate build.rs-free declarative authoring
- [x] #4 Regression tests verify clean compilation and execution of applications without build scripts
<!-- AC:END -->
