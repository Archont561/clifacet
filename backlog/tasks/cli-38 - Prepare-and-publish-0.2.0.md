---
id: CLI-38
title: Prepare and publish 0.2.0
status: To Do
assignee: []
created_date: '2026-11-01 22:46'
labels: []
milestone: m-10
dependencies:
  - CLI-20
type: task
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run the second release through the runbook that was built for the first, including the accepted criteria of 0.1.0 that could only be proved later. Packaging, MSRV, feature matrix, both platforms and the ordered trusted publication all pass before the tag exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 The version is bumped through the runbook with `--relock`, and the lockfile change comes from the bot step rather than a hand edit.
- [ ] #2 Every quality gate passes on the release commit before the tag exists.
- [ ] #3 The changelog and the release notes are rendered from the commits; no section is written by hand.
- [ ] #4 Both crates publish from one authorized dispatch through the `crates` environment, in order, with the tag matching the published version.
- [ ] #5 The remaining acceptance criterion of CLI-20 is satisfied here: the relock path is exercised from a `main` dispatch, and the evidence is recorded on that card.
<!-- AC:END -->
