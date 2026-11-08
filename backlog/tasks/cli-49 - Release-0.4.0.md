---
id: CLI-49
title: Release 0.4.0
status: To Do
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 11:15'
labels: []
milestone: m-12
dependencies:
  - CLI-48
  - CLI-55
type: task
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release that closes M12: the runner, the application context and `clifacet::commands!`. A minor bump because the public surface grows and decision-14's `Request` rule changes for the `commands!` path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `bump-version 0.4.0 --relock` lands through a PR whose lock guard and msrv jobs are green
- [ ] #2 The maintainer dispatches release.yml with publishing enabled and both crates are on crates.io at 0.4.0, with the tag, notes and changelog produced by the workflow
- [ ] #3 The run URL is recorded here
<!-- AC:END -->
