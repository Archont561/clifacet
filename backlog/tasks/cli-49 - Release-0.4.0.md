---
id: CLI-49
title: Release 0.4.0
status: Done
assignee: []
created_date: '2026-11-08 01:59'
updated_date: '2026-11-08 18:41'
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
- [x] #1 `bump-version 0.4.0 --relock` lands through a PR whose lock guard and msrv jobs are green
- [x] #2 The maintainer dispatches release.yml with publishing enabled and both crates are on crates.io at 0.4.0, with the tag, notes and changelog produced by the workflow
- [x] #3 The run URL is recorded here
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-08: 0.4.0 shipped and the card was left open, which is a bookkeeping fault worth naming rather than quietly correcting: every criterion was already satisfied when the work finished, and nothing in the tooling notices a task whose criteria are all met but whose status is not `Done`. The evidence, in the order the criteria ask for it:

- **AC#1** `bump-version 0.4.0 --relock` landed as PR #69 (`chore(release): record 0.4.0 in every manifest`, merge `bad2dc83`). Every gate on that PR passed, including `lock guard`; the `relock` job skipped because the bot had nothing left to change, and `relock verdict` skipped with it.
- **AC#2** the maintainer dispatched `release.yml` (run [42496181369](https://github.com/Archont561/clifacet/actions/runs/42496181369), `Publish to crates.io` succeeded, 2026-11-08T15:42:39Z). Tag `v0.4.0` is `bad2dc83` on the remote, GitHub Release `v0.4.0` was published at 15:43:35Z, and `CHANGELOG.md` carries `## [0.4.0] - 2026-11-08`. `bump-version --check` reports all three version sites at 0.4.0.
- **AC#3** the run URL is recorded in this note, which is what the criterion asked for.

The lesson is not that the release was wrong — it was right, and it matched the runbook. It is that closing a release card is a step that has to be done by hand, so it was added to the release runbook as one.
<!-- SECTION:NOTES:END -->
