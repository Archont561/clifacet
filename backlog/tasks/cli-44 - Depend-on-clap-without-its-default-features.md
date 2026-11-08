---
id: CLI-44
title: Depend on clap without its default features
status: Done
assignee: []
created_date: '2026-11-08 01:58'
updated_date: '2026-11-08 12:18'
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
- [x] #1 The workspace clap dependency sets `default-features = false` and enables only `std`, `help`, `usage`, `error-context`, `derive` and `env`
- [x] #2 Both examples enable `color` and `suggestions` themselves, as an application would, so their help and errors are unchanged
- [x] #3 `cargo tree -p clifacet -e normal` resolves none of the colour packages; the lockfile keeps them only because the examples ask for them
- [x] #4 test-local is green, and the change is a mechanical dependency edit recorded as such (no new scenario)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Mechanical dependency edit (AGENTS.md exception): no behaviour of clifacet changes, so there is no red test; the evidence is the dependency graph and the existing suites on the lean features alone.

- `cargo tree --offline -p clifacet -e normal`: no anstream, anstyle-query, colorchoice, strsim or utf8parse.
- `cargo tree --offline -p clifacet-example -e normal`: anstream, colorchoice and strsim are back, because the example asks for `color` and `suggestions`. That is also why Cargo.lock does not change: the lock covers the workspace, and the examples still need those packages.
- `cargo check -p clifacet --no-default-features`, `cargo clippy -p clifacet -p clifacet-macros --all-features --all-targets -D warnings`, and `cargo test -p clifacet --all-features` (every suite including the 6 trybuild cases) pass with clap resolved for clifacet alone, so no feature is unified back on by an example.

Compatibility: an application that depends on clap with its default features (the usual `clap = "4"`) keeps colour and suggestions, because Cargo unifies features. Only an application that relied on clifacet to switch them on loses them, and gets them back by adding the two features to its own clap dependency.

test-local green on the branch with this change: 485 integration and 82 unit tests passed, plus deny, check-advisory, check-packages and sandbox-check.
<!-- SECTION:NOTES:END -->
