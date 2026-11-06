---
id: CLI-30
title: Choice and classified-confirmation questions
status: In Progress
assignee: []
created_date: '2026-11-01 22:34'
updated_date: '2026-11-06 09:41'
labels: []
milestone: m-6
dependencies: []
type: task
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Session and Prompter can ask a multi-choice question, a secret, and a repeated value, and can tell an answered confirmation from one nothing could ask. Decision-18 items 1-3; the backlog-lite wrapper is the acceptance case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 `Prompter::choose` takes labelled choices and `Session::choose` returns a supplied value untouched, so prompting never overrides an explicit answer.
- [ ] #2 The `inquire` frontend offers the choices as a selectable list, and the scripted frontend answers them, with a behaviour test for both.
- [ ] #3 `Session::ask_secret` asks without echoing, records `Source::Prompt` with the secret flag set, and the value is reachable only through `Secret`.
- [ ] #4 `Session::collect_values` repeats one question until the user stops, and a partially collected list is never returned as success.
- [ ] #5 `Session::confirm` classifies the outcome as asked-and-declined, assumed by `--yes`, or unavailable, while the boolean fail-safe behaviour is unchanged.
- [ ] #6 `examples/backlog-lite` stops inspecting `session.plan()` by hand and its status filter becomes a choice prompt; its tests stay green or get simpler.
<!-- AC:END -->
