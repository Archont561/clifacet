---
id: CLI-53
title: Release 0.5.0
status: To Do
assignee: []
created_date: '2026-11-08 02:00'
labels: []
milestone: m-13
dependencies:
  - CLI-50
  - CLI-51
  - CLI-52
type: task
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release that closes M13: discovered commands and helper modules, layered settings, and functions as commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `bump-version 0.5.0 --relock` lands through a PR whose lock guard and msrv jobs are green
- [ ] #2 The maintainer dispatches release.yml with publishing enabled and both crates are on crates.io at 0.5.0, with the tag, notes and changelog produced by the workflow
- [ ] #3 The run URL is recorded here
<!-- AC:END -->
