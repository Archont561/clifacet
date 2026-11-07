---
id: CLI-42
title: Library-level BDD scenarios in clifacet
status: In Progress
assignee: []
created_date: '2026-11-07 22:51'
updated_date: '2026-11-07 22:51'
labels: []
milestone: m-7
dependencies: []
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested on 2026-11-07 ("add few features to clifacet and when needed"): every .feature file lives in the example crates since CLI-21 moved the reference application out of the library, so cargo test -p clifacet runs no scenario and some library promises have none anywhere — the nested command tree (CLI-32) above all. Add a few scenarios to crates/clifacet/tests/features, driven through the library's own public seams (clap parse, Collect on a scripted Session, Facet::validate, Command/Planned, Provenance) without an application. clifacet-macros gets a scenario only where one would describe something trybuild does not already pin. Tests only: no production code changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/clifacet/tests/features holds feature files for the command tree, planned commands and provenance, with rstest-bdd step bindings in crates/clifacet/tests, and cargo test -p clifacet runs them
- [ ] #2 Every step drives a public seam a consumer uses; no scenario restates an existing example scenario
- [ ] #3 Each scenario is shown able to fail (a deliberately wrong expectation fails, then is restored), and the notes say plainly they were written after the behaviour existed
- [ ] #4 Whether clifacet-macros needs scenarios is decided and the reason recorded
- [ ] #5 doc-4 says where scenarios live: library promises in crates/clifacet, application workflows in the example crates, macro diagnostics in trybuild
<!-- AC:END -->
