---
id: CLI-32
title: Nested commands as a declarative tree
status: In Progress
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 20:01'
labels: []
milestone: m-7
dependencies:
  - CLI-31
type: task
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A command whose subcommands are declared where they live instead of assembled by hand. Decision-14 (accepted 2026-11-06, with amendments) fixes the shape: a leaf is a struct, a tree is an enum whose variants wrap leaves, clap still owns parsing and every input is still a typed value, while the caller writes the nesting once. `Request` stays hand-written and the generated code checks it. Built after CLI-31 and the 0.3.0 release, because the tree's generated `Command` impl delegates every method of the trait, `plan` included. The reference is `examples/backlog-lite`, the repository's only tree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A command tree is declared where the commands live and clap still owns parsing, so every value a command receives is a validated typed input.
- [x] #2 A leaf three levels deep parses, and its help shows the full path, for example `cast db migrate --help`.
- [x] #3 Conversion from `ArgMatches` works at every node, not only at the root, and the tree's generated `Command` impl delegates every method of the trait to the chosen leaf.
- [x] #4 A root may hold `Option<Commands>`; an absent subcommand reaches the application, which chooses any default leaf itself.
- [x] #5 `Request` is written by the application, and a missing, extra or mistyped `Request` variant fails to compile, covered by trybuild.
- [x] #6 A tree that repeats a name, uses a key the surface does not define, or has an inline-field or unit variant fails to compile with a message naming the offending key or saying to write a leaf struct, covered by trybuild cases.
- [x] #7 `examples/backlog-lite` declares its tree instead of writing the clap enum and the dispatch by hand, its tests are unchanged, and the line count before and after is recorded in the notes for decision-15's re-measurement.
- [x] #8 The nesting rule is part of the command authoring guide, with the compiled snippet check-docs already requires.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Design (maintainer's choices, 2026-11-06): leaf structs are the commands; the tree's generated Command matches (tree, request) pairs and runs the leaf's own Command, a mismatched pair being Failure::execution; generated only with output = X. With planned as well the tree generates Planned instead, delegating plan through the new Plan::map, and Command comes from the blanket impl. collect = path on #[clifacet::command] mirrors bridge for questions the declarations cannot describe.

Surface: #[clifacet::subcommands(input = Request [, output = X [, planned]])] on an enum of Name(Leaf) variants, with variant keys name, alias, visible_alias and hide; #[clifacet(subcommands)] on a field holding the tree or an Option of it; collect = path; Plan::map. Facet and Collect gained diagnostic::on_unimplemented messages. heck is now a direct macro dependency (already in every build through clap), so the duplicate-name check computes default names as clap does.

Evidence: crates/clifacet/tests/command_tree.rs (18 tests, cast db migrate three levels deep: help shows Usage: cast db migrate, conversion from each node's own ArgMatches, Option root reaching the application as None, Command/Planned/Collect delegation); 15 trybuild cases in crates/clifacet/tests/compile_fail.rs (missing, extra and mistyped Request variant; leaf without Facet; output that does not convert; repeated name; unknown tree and variant keys; inline and unit variants; planned without output; misuse of subcommands and collect); 15 macro unit tests. Full suite 556 passed, source line coverage 98.45%.

AC#8: there is no page titled command authoring guide; the rule went into docs/derive.md (The companion derive), where #[clifacet::command] authoring is documented, as section Nested commands: a tree declared where its commands live, with a compiled three-level snippet.

AC#7 line counts for decision-15's re-measurement (examples/backlog-lite, tests unchanged: git diff of tests/ is empty): before (adab86a8) lib.rs 717 + main.rs 17 = 734; after lib.rs 709 + main.rs 17 = 726 (-8). Without blank and comment lines, lib.rs 503 -> 472 (-31, about -6%). The hand-written TryFrom<Cli> and collect dispatch (about 110 lines) are gone; most of the saving is spent on one named input struct per leaf (Filter, TaskRef, NewTask, Query, Checkup) where Action held inline fields. One message changed wording: a blank show ID or search query now reports the generated non_empty message instead of the hand-written one (start and done still check the ID with task_id while confirming); no test asserts either.
<!-- SECTION:NOTES:END -->
