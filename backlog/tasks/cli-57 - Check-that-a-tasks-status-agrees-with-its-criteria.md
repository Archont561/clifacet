---
id: CLI-57
title: Check that a task's status agrees with its criteria
status: Done
assignee: []
created_date: '2026-11-08 18:43'
updated_date: '2026-11-08 18:43'
labels: []
milestone: m-13
dependencies: []
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
backlog doctor validates the graph but never a status. Three cards drifted in one day: CLI-49 and CLI-50 finished and stayed open, CLI-38 stayed Done with a criterion unchecked because its proof was recorded on CLI-20. A board that reads as finished work has to be checked, or it is decoration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 backlog-check fails for a card in Done whose acceptance criteria are not all checked
- [x] #2 backlog-check fails for a card whose criteria are all checked but whose status is not Done
- [x] #3 Only boxes inside the AC block count, and a card with no criteria is not checked; both are covered by tests
- [x] #4 AGENTS.md says closing the card is part of the work, and docs/development.md explains the two checks
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-08: found while asking "what now?" after PR #71 merged. The board said M13 was untouched, but CLI-49's release had shipped hours earlier and CLI-50's six criteria had all been met — and the reverse case sat in plain sight too, a `Done` card with an open criterion. The commands available (`backlog task list`, `backlog doctor`) have nothing to say about either: doctor validates the graph, and `definition_of_done` only seeds new tasks. So this was not a matter of remembering to run something; nothing existed to run.

Three findings, three answers, in the same change:

- **CLI-49** marked Done, with the bump PR (#69), the release run, the tag, the notes and the changelog section recorded — its third criterion literally asked for the run URL, so the note is the evidence rather than a summary of it.
- **CLI-50** marked Done, with a note saying its criteria were proven when the implementation landed and only the status change was missing.
- **CLI-38's** AC#5 checked. It waited on CLI-20's AC#4, which was proven on 2026-11-08; the proof was recorded on CLI-20 because that is where the criterion lived, and nothing carried it back.

**The guard.** `scripts/check_backlog_status.py` runs after `backlog doctor` inside `backlog-check`, so it is in `quality`, the documentation lane, and `test-local` without a new task or a new CI step. Two rules, one per direction of drift.

Two judgement calls, both deliberate and both tested. Only boxes between `AC:BEGIN` and `AC:END` count — a tick in the notes is prose about the work, and counting it would let a card pass by mentioning a checkbox. And a card with no criteria is skipped: several are notes to self, and a rule for them would be a rule about the tool rather than the work.

**A bug worth naming,** because the fix is in the file: `main()` took a `directory` argument and then ignored it, reading the repository's own backlog regardless. The first test run "passed" against a planted-fault tree for exactly that reason — the tree it checked was the clean one. Found by testing the guard against a copy with faults planted, which is the only way a check that must always pass on this repository can be shown to fire at all.

Verified by hand: a planted `Done`-with-open-criterion card and a planted all-met-but-open card both report, with the id, the counts and the two ways out.
<!-- SECTION:NOTES:END -->
