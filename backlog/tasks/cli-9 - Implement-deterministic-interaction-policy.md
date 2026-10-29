---
id: CLI-9
title: Implement deterministic interaction policy
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-2
dependencies:
  - CLI-7
type: task
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Resolve runtime mode based on explicit request, app policy and capabilities.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Policy is deterministic and covered by rstest decision tables.
- [ ] #2 Forbidden modes do not construct a prompter; yes affects confirmations only.
- [ ] #3 Cancellation and invalid explicit requests are distinct and do not execute the handler.
<!-- AC:END -->
