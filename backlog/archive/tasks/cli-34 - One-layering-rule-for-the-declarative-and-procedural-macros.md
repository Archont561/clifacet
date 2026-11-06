---
id: CLI-34
title: One layering rule for the declarative and procedural macros
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-06 14:30'
labels: []
milestone: m-7
dependencies:
  - CLI-28
type: task
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
State where the declarative surface ends and a procedural macro begins, and enforce it, so a reader never has to guess which of two spellings is canonical. Decision-16 is the rule; this card makes the crate follow it and adds the compile-fail coverage that proves the boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 One written rule says which layer owns which syntax and why, and the crate's own code follows it.
- [ ] #2 Every public macro states which layer it belongs to and what it expands to, in its documentation.
- [ ] #3 Crossing the boundary — a field attribute where a command is expected, or the reverse — fails to compile with a message that says what to write instead, covered by trybuild.
- [ ] #4 A reader can answer "which macro do I need?" from the authoring guide alone, without opening the source.
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Folded into CLI-33 on 2026-11-06 (decision-16 accepted, narrowed to the rule): the layering criteria apply to the first exported `macro_rules!`, and with decision-15's `commands!` deferred, CLI-33 is the card that would ship one. Its four criteria are carried as CLI-33's last two. Archived, not done: no work was performed on this card.
<!-- SECTION:FINAL_SUMMARY:END -->
