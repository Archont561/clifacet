---
id: CLI-2
title: Add Alint and tested Python repository checks
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Separate declarative repository policy from project-specific semantic checks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Alint checks required files and workflow policy with a reviewed baseline if needed.
- [ ] #2 Python checks have pytest coverage and do not duplicate policy parsers owned by other tools.
- [ ] #3 CI and local gates call the same documented Pixi entrypoints.
<!-- AC:END -->
