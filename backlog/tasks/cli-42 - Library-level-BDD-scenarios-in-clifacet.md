---
id: CLI-42
title: Library-level BDD scenarios in clifacet
status: In Progress
assignee: []
created_date: '2026-11-07 22:51'
updated_date: '2026-11-07 22:58'
labels: []
milestone: m-7
dependencies: []
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested on 2026-11-07 ("add few features to clifacet and when needed"): every .feature file lives in the example crates since CLI-21 moved the reference application out of the library, so cargo test -p clifacet runs no scenario and some library promises have none anywhere — the nested command tree (CLI-32) above all. Add a few scenarios to crates/clifacet/features, driven through the library's own public seams (clap parse, Collect on a scripted Session, Facet::validate, Command/Planned, Provenance) without an application. clifacet-macros gets a scenario only where one would describe something trybuild does not already pin. Tests only: no production code changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 crates/clifacet/features holds feature files for the command tree, planned commands and provenance, with rstest-bdd step bindings in crates/clifacet/tests, and cargo test -p clifacet runs them
- [x] #2 Every step drives a public seam a consumer uses; no scenario restates an existing example scenario
- [x] #3 Each scenario is shown able to fail (a deliberately wrong expectation fails, then is restored), and the notes say plainly they were written after the behaviour existed
- [x] #4 Whether clifacet-macros needs scenarios is decided and the reason recorded
- [x] #5 doc-4 says where scenarios live: library promises in crates/clifacet, application workflows in the example crates, macro diagnostics in trybuild
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**What was added.** Three feature files under crates/clifacet/features with ten scenarios, and rstest-bdd step bindings next to them (rstest-bdd and rstest-bdd-macros are now dev-dependencies of clifacet; Cargo.lock gains only those two lines in clifacet's entry):

- command_tree.feature (4) in command_tree_bdd.rs: values from every level of a three-level tree reach the leaf; only the chosen leaf asks its own question (a collect = path confirmation); running the tree runs only the chosen leaf; with no subcommand the application gets None and nothing is asked. This is the first scenario for decision-14/CLI-32 anywhere in the repository.
- plan.feature (3) in plan_bdd.rs: a preview lists every step and changes nothing; a real run, through the Command impl the library provides for every Planned command, deploys exactly what the preview listed; a request refused while planning runs nothing.
- provenance.feature (3) in provenance_bdd.rs: a typed value, a default and an environment value are told apart; a value nobody supplied has no source; an answer to a declared ask is recorded as prompt, not argv, and the record holds no value.

**Public seams only (AC#2).** Steps use clap's parse, Collect on a scripted Session, Facet::validate, Command::execute, Planned::plan/Plan::steps and Provenance::from_matches/merge, the same calls an application makes. No example scenario is restated: backlog-lite's plan.feature covers bl --dry-run output, while this one covers the library's preview-equals-run promise without an application. secret.feature records provenance by hand, while this one shows the declared command and session recording it.

**Red evidence, stated plainly (AC#3).** These scenarios describe behaviour that already existed, so they were not red first. To show each one can fail, one expectation per scenario was changed in the feature files (for example --steps 3 instead of 2, "Drop some tables?", "deploy api", repeat from argv). All 10 failed (nextest: 0 passed, 10 failed). Nine failed on their own assertion. without_a_subcommand, mutated to run "cast greet" at a terminal, failed one step earlier because an unscripted question made the run fail validation; that is still a behavioural failure. The files were then restored and all 10 pass.

**clifacet-macros: no scenarios (AC#4).** Its generated code names ::clifacet, so it can only be exercised from a crate that depends on clifacet, and the new command_tree scenarios already drive #[clifacet::command] and #[clifacet::subcommands] end to end. Its other promises are compile-time diagnostics, which decision-6 assigns to trybuild (44 cases in crates/clifacet/tests/ui). A Gherkin wrapper would only restate those .stderr snapshots, which the bdd skill advises against.

**Docs (AC#5).** doc-4 gained a "Where scenarios live" section: library promises in crates/clifacet, application workflows in the example crates, and macro diagnostics in trybuild.

**Design.** Tests only, with no production code change. Each step file declares its own small world, so the scenarios stay independent. The command-tree file declares its own tree instead of sharing command_tree.rs's, because integration test files are separate crates.


**Layout change (2026-11-08, requested by the maintainer).** Feature files moved from `<crate>/tests/features/` to `<crate>/features/` in all three crates (clifacet, clifacet-example, clifacet-backlog-lite), with `git mv` so history follows. Step bindings stay in `tests/*_bdd.rs`, and their 43 `#[scenario(path = ..)]` attributes now read `features/..`. clifacet's package `exclude` is now `features/`. AGENTS.md, the bdd skill, doc-4, docs/testing.md, docs/command-core.md, docs/derive.md, the simple README and two script-test fixtures follow. Historical task notes keep the paths that were true when they were written.
<!-- SECTION:NOTES:END -->
