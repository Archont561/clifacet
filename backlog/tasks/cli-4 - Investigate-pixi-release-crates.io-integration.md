---
id: CLI-4
title: Investigate pixi-release crates.io integration
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Determine a supported generated release path for the two-crate trusted-publishing contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inspect actual pixi-release target and workflow capabilities and record the chosen integration or required upstream gap.
- [ ] #2 Release plan accounts for `release.yml`, environment `crates`, OIDC, macro-first order and partial-publish recovery.
- [ ] #3 No live publish is required or performed for this investigation.
<!-- AC:END -->
