---
id: CLI-47
title: Give commands the application's context
status: To Do
assignee: []
created_date: '2026-11-08 01:59'
labels: []
milestone: m-12
dependencies:
  - CLI-46
type: task
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 2. Inside a tree, `self` is the parsed clap struct, so a command has nowhere to receive a store, a client or a clock: backlog-lite reads a global CLIFACET_BACKLOG_BIN variable and the tree tests record runs in a thread_local!. A context is passed by shared reference beside the input; `Command` itself does not change. The coherence of the blanket impl across crates and the E0119 for a type implementing both traits were compiled during the review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `Handler<Cx>` with `handle(&self, input, cx: &Cx)` is implemented for every `Command` by a blanket impl, so no existing implementation changes, proven by a test crate outside clifacet; implementing both traits for one type is a documented compile error
- [ ] #2 A planned twin lets a command plan with the context, and the plan's work may borrow from it
- [ ] #3 `#[clifacet::subcommands(.., context = T)]` threads `&T` to the chosen leaf, and a leaf without context still runs inside such a tree
- [ ] #4 The runner builds the context after validation, on first use, and never for `--help`, `--version`, a usage error or a declined question; a context that cannot be built is an execution failure and nothing runs (scenarios)
- [ ] #5 A question can read the context, for example to offer existing records as choices
- [ ] #6 The authoring guide explains context versus input — dependencies versus what the user supplied — with a compiled snippet
<!-- AC:END -->
