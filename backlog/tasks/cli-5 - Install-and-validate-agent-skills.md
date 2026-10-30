---
id: CLI-5
title: Install and validate agent skills
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Adopt a consistent skill source/lock model and create the requested local direction-reset skill.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reviewed session, backlog, testing and relevant supporting skills are installed with a documented source/lock model.
- [x] #2 Local regain-direction skill follows doc-6 and does not perform destructive or publication actions without authorization.
- [x] #3 A tested skills-check is part of the canonical gate.
<!-- AC:END -->

## Notes

- Verified: 11 local skills validate, `skills-check` is part of the `quality` task and CI, and `regain-direction` follows doc-6 including the rule against closing tasks or publishing without authorization.
