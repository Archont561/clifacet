---
id: CLI-45
title: Release 0.3.1
status: Done
assignee: []
created_date: '2026-11-08 01:58'
updated_date: '2026-11-08 12:18'
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
- [x] #1 `bump-version 0.3.1 --relock` lands through a PR whose lock guard and msrv jobs are green
- [x] #2 The maintainer dispatches release.yml from main with "Package and verify without publishing" unchecked, and both crates are on crates.io at 0.3.1
- [x] #3 The tag v0.3.1, its GitHub Release notes and the regenerated CHANGELOG.md come from the workflow, rendered from the commits
- [x] #4 The run URL is recorded here and noted on CLI-20 as the end-to-end rehearsal its criterion #4 asked for
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-08: released end to end.
- AC#1: `bump-version 0.3.1 --relock` landed in PR #60 (merge 7a3696e9); its lock guard and msrv jobs passed, and main CI on the merge (run 42438878544) passed.
- AC#2: the maintainer dispatched release.yml from main with the dry-run box unchecked: run [42473663383](https://github.com/Archont561/clifacet/actions/runs/42473663383), success, with the dry-run report step skipped. crates.io lists clifacet-macros 0.3.1 and clifacet 0.3.1, both trusted-published with run_id 42473663383 and sha 7a3696e9.
- AC#3: tag `v0.3.1` points at 7a3696e9, the GitHub Release v0.3.1 carries the notes rendered from the commits (Added, Fixed, Changed), and 1b7a1c4 `chore(release): regenerate the changelog` by github-actions[bot] put `## [0.3.1] - 2026-11-08` on main by itself.
- AC#4: run URL recorded here and on CLI-20.
Status left In Progress: closing needs maintainer authorisation.
<!-- SECTION:NOTES:END -->
