---
id: CLI-45
title: Release 0.3.1
status: To Do
assignee: []
created_date: '2026-11-08 01:58'
labels: []
milestone: m-11
dependencies:
  - CLI-43
  - CLI-44
type: task
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The patch release for CLI-43 and CLI-44, cut with the runbook in docs/development.md. It is also the real version bump that CLI-20's last criterion (#4, a full release rehearsed end to end) asked for, so its run is recorded there as evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `bump-version 0.3.1 --relock` lands through a PR whose lock guard and msrv jobs are green
- [ ] #2 The maintainer dispatches release.yml from main with "Package and verify without publishing" unchecked, and both crates are on crates.io at 0.3.1
- [ ] #3 The tag v0.3.1, its GitHub Release notes and the regenerated CHANGELOG.md come from the workflow, rendered from the commits
- [ ] #4 The run URL is recorded here and noted on CLI-20 as the end-to-end rehearsal its criterion #4 asked for
<!-- AC:END -->
