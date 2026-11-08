---
id: CLI-44
title: Depend on clap without its default features
status: In Progress
assignee: []
created_date: '2026-11-08 01:58'
labels: []
milestone: m-11
dependencies: []
type: task
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The workspace enabled clap's default features, so every application using clifacet got clap's colour and "did you mean" suggestions whether it wanted them or not, with eight packages in the lock that the library never uses (anstream, anstyle-parse, anstyle-query, anstyle-wincon, colorchoice, is_terminal_polyfill, once_cell_polyfill, utf8parse). The library code uses no colour or suggestion API. Measured in the decision-19 review: the whole workspace compiles and all 585 tests pass with them off. Cargo unifies features, so an application that depends on clap with its defaults keeps both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The workspace clap dependency sets `default-features = false` and enables only `std`, `help`, `usage`, `error-context`, `derive` and `env`
- [ ] #2 Both examples enable `color` and `suggestions` themselves, as an application would, so their help and errors are unchanged
- [ ] #3 `cargo tree -p clifacet -e normal` resolves none of the colour packages; the lockfile keeps them only because the examples ask for them
- [ ] #4 test-local is green, and the change is a mechanical dependency edit recorded as such (no new scenario)
<!-- AC:END -->
