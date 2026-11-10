---
id: CLI-53
title: Release 0.5.0
status: In Progress
assignee: []
created_date: '2026-11-08 02:00'
updated_date: '2026-11-10 01:40'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-10: the bump is recorded. `bump-version 0.5.0 --relock` rewrote the three version
sites (`Cargo.toml` `[workspace.package]`, its `clifacet-macros` requirement, and
`pixi.toml` `[workspace]`) and refreshed `Cargo.lock`; the README was re-rendered from its
template, so the install snippet and the status line read 0.5.0 rather than 0.4.0.

M13's three dependencies — CLI-50 (discover a folder's commands), CLI-51 (layer settings
from a file, the environment and flags) and CLI-52 (write a command as a function) — are
`Done`, and the notes for 0.5.0 render from the commits since `v0.4.0`, so the release has
something to describe. No `CHANGELOG.md` entry appeared at bump time, which is expected:
an `### Added` entry belongs to the commit that adds the feature, and this card's bump only
records a version.
<!-- SECTION:NOTES:END -->
