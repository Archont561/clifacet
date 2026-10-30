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

## Notes

- 2026-10-30: two readiness gaps were found before this task's rehearsal and are now fixed in the
  release path itself: the first release could not date its changelog (`bump-version
  --promote-current` now does), and `release.yml` resolved the notes only after publishing (it now
  resolves them before anything is packaged, so a missing section fails with nothing to revert, and
  the tag step reuses the verified file). Both are recorded with their tests under CLI-20.
- Still unproven without a real run: AC#2's "registry availability" and "partial-failure recovery",
  which need a first real publish, and AC#3's authorization record. `--promote-current` must be run
  and merged before the rehearsal dispatch, because a dry run now checks the notes too.
