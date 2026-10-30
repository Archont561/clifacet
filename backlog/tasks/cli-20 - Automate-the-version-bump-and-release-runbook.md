---
id: CLI-20
title: Automate the version bump and release runbook
status: In Progress
assignee: []
created_date: '2026-10-30 16:30'
labels: []
milestone: m-5
dependencies:
  - CLI-4
type: task
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Turn "release a new version" from a multi-file manual chore into one command with notes, a tag and a GitHub Release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `scripts/bump_version.py` rewrites every recorded version site, with pytest coverage and no network.
- [x] #2 The changelog is promoted by the same command, and an empty `Unreleased` section blocks the bump.
- [x] #3 `release.yml` tags the release and attaches the changelog notes only after both crates publish.
- [ ] #4 A full release is rehearsed end to end on a real version bump.
<!-- AC:END -->

## Notes

- 2026-10-30: #1 to #3 landed. The script rewrites the workspace version, the `clifacet-macros` requirement and the `pixi.toml` version, promotes `Unreleased` to a dated section, moves the changelog link references, and can relock `Cargo.lock` offline (`--relock`). `--check` verifies the three sites agree and is now called by `release.yml`; `--notes` feeds the GitHub Release body.
- #4 stays open deliberately: the release path has never published, and exercising it needs a real version bump plus maintainer authorization. CLI-18 owns that rehearsal.
- The tag and release step is idempotent: an existing `v<version>` release is left untouched, so a re-run after a partial publish does not fail.
