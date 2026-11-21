---
id: CLI-92
title: Route Clap derives through clifacet re-export for single-dependency authoring
status: Done
assignee: []
created_date: '2026-11-21 08:08'
updated_date: '2026-11-21 08:15'
labels: []
milestone: m-23
dependencies: []
type: enhancement
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Update procedural macro expansions in clifacet-macros to resolve Clap traits, derives, and command factories via ::clifacet::clap with #[command(crate = "::clifacet::clap")] so downstream crates only need clifacet in Cargo.toml.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Macros #[clifacet::app], #[clifacet::commands], #[clifacet::command], and #[clifacet::subcommands] emit Clap derives with crate = "::clifacet::clap"
- [x] #2 CommandFactory and Subcommand trait calls reference ::clifacet::clap instead of top-level ::clap
- [x] #3 Examples and test harnesses compile and run without needing direct clap dependency
<!-- AC:END -->
