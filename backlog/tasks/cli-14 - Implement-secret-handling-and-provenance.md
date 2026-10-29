---
id: CLI-14
title: Implement secret handling and provenance
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-4
dependencies:
  - CLI-7
  - CLI-13
type: task
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define and test framework-owned secret and provenance guarantees.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Framework diagnostics, previews and test records redact supplied secret values.
- [ ] #2 Input provenance records source without the secret value; failed tests do not print canaries.
- [ ] #3 Limitations for application-owned logs and unsafe argv sources are documented.
<!-- AC:END -->
