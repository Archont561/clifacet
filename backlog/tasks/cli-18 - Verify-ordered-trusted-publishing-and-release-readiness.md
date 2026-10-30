---
id: CLI-18
title: Verify ordered trusted publishing and release readiness
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-5
dependencies:
  - CLI-4
  - CLI-16
  - CLI-17
type: task
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rehearse the authored two-crate publication contract end to end.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `release.yml` publishes macro-first using environment `crates`, job-scoped OIDC and pinned official actions.
- [ ] #2 Macro-first publication, registry availability, partial-failure recovery and package contents are verified without an unauthorized live release.
- [ ] #3 Release is performed only after explicit maintainer authorization and recorded successful gates.
<!-- AC:END -->
