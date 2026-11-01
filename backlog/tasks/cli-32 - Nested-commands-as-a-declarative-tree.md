---
id: CLI-32
title: Nested commands as a declarative tree
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
labels: []
milestone: m-7
dependencies:
  - CLI-28
type: task
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A command whose subcommands are declared where they live instead of assembled in a builder. Decision-14 fixes the shape: the tree is a value the derive can read, so clap still owns parsing and every input is still a typed value, while the caller writes the nesting once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 A command tree is declared where the commands live and clap still owns parsing, so every value a command receives is a validated typed input.
- [ ] #2 A leaf three levels deep parses, and its help shows the full path, for example `cast db migrate --help`.
- [ ] #3 Conversion from `ArgMatches` works at every node, not only at the root.
- [ ] #4 A tree that repeats a name, or uses a key the surface does not define, fails to compile with a message naming the offending key, covered by a trybuild case.
- [ ] #5 `examples/simple` declares its tree instead of assembling it, and its parity tests are unchanged.
- [ ] #6 The nesting rule is part of the command authoring guide, with the compiled snippet check-docs already requires.
<!-- AC:END -->
