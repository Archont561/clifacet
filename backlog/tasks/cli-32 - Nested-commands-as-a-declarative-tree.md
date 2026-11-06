---
id: CLI-32
title: Nested commands as a declarative tree
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 14:30'
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
- [ ] #1 A command tree is declared where the commands live and clap still owns parsing, so every value a command receives is a validated typed input.
- [ ] #2 A leaf three levels deep parses, and its help shows the full path, for example `cast db migrate --help`.
- [ ] #3 Conversion from `ArgMatches` works at every node, not only at the root, and the tree's generated `Command` impl delegates every method of the trait to the chosen leaf.
- [ ] #4 A root may hold `Option<Commands>`; an absent subcommand reaches the application, which chooses any default leaf itself.
- [ ] #5 `Request` is written by the application, and a missing, extra or mistyped `Request` variant fails to compile, covered by trybuild.
- [ ] #6 A tree that repeats a name, uses a key the surface does not define, or has an inline-field or unit variant fails to compile with a message naming the offending key or saying to write a leaf struct, covered by trybuild cases.
- [ ] #7 `examples/backlog-lite` declares its tree instead of writing the clap enum and the dispatch by hand, its tests are unchanged, and the line count before and after is recorded in the notes for decision-15's re-measurement.
- [ ] #8 The nesting rule is part of the command authoring guide, with the compiled snippet check-docs already requires.
<!-- AC:END -->
