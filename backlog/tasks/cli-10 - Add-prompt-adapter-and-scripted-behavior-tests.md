---
id: CLI-10
title: Add prompt adapter and scripted behavior tests
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-2
dependencies:
  - CLI-9
type: task
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provide an optional interactive adapter that shares the command core.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Missing values are prompted only when policy permits; supplied values are not needlessly re-asked.
- [ ] #2 A scripted framework-owned seam tests successful input, cancellation and validation.
- [ ] #3 Representative BDD scenarios demonstrate flag and prompt parity.
<!-- AC:END -->
