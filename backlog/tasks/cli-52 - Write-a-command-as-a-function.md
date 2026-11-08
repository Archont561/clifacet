---
id: CLI-52
title: Write a command as a function
status: To Do
assignee: []
created_date: '2026-11-08 02:00'
labels: []
milestone: m-13
dependencies:
  - CLI-47
  - CLI-33
type: task
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 6, the last piece of the authoring surface: `#[clifacet::command] pub fn add(title: String, cx: &App) -> Result<usize, Failure>`. A parameter typed as a reference to the application context is the context; every other parameter is input, and the generated struct is what clap parses. Comes after the runner, the context and registration because it is sugar over all three.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A function annotated `#[clifacet::command]` becomes a command whose arguments are its non-context parameters, with doc comments as help and `#[clifacet(..)]` field keys on parameters
- [ ] #2 A parameter typed `&Cx` receives the application context; more than one context parameter is a compile error that says why
- [ ] #3 A function command registers and dispatches exactly like a struct command, explicitly and through discovery
- [ ] #4 trybuild cases cover a non-`Result` return type and an unsupported parameter pattern, and the guide shows both forms side by side
<!-- AC:END -->
