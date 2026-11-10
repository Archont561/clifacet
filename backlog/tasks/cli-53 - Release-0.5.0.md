---
id: CLI-53
title: Release 0.5.0
status: Done
assignee: []
created_date: '2026-11-08 02:00'
updated_date: '2026-11-10 02:05'
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
- [x] #1 `bump-version 0.5.0 --relock` lands through a PR whose lock guard and msrv jobs are green
- [x] #2 The maintainer dispatches release.yml with publishing enabled and both crates are on crates.io at 0.5.0, with the tag, notes and changelog produced by the workflow
- [x] #3 The run URL is recorded here
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

2026-11-10: 0.5.0 shipped. The evidence, in the order the criteria ask for it:

- **AC#1** `bump-version 0.5.0 --relock` landed as PR #78 (`chore(release): record 0.5.0 in
  every manifest`, merge `f9e59b16`). All nine checks passed, including `lock guard` and
  `Minimum supported Rust version`; `relock` and `relock verdict` skipped because the bot
  had nothing left to change. `bump-version --check` reports all three sites at 0.5.0.
- **AC#2** a rehearsal was dispatched first (run
  [42738023620](https://github.com/Archont561/clifacet/actions/runs/42738023620)): preflight
  and packaging succeeded and the four irreversible steps were skipped, with no tag and no
  release created. The release followed as run
  [42739061458](https://github.com/Archont561/clifacet/actions/runs/42739061458) with
  `dry_run` unchecked — the OIDC exchange, the publish loop, the tag and the changelog step
  all succeeded. Tag `v0.5.0` is `f9e59b16`, the GitHub Release was published at
  01:55:26Z with the notes preflight verified, and `main` carries `1e8c95df`
  `chore(release): regenerate the changelog`, whose `## [0.5.0] - 2026-11-10` section passes
  `changelog-check` — the 0.3.1 regression did not recur.
- **AC#3** the run URLs are recorded above.

One thing to note for the next release: nothing in the tree named `0.4.0` outside the
manifests and `Cargo.lock` except `README.md`, which is now generated, so re-running
`pixi run readme` was the whole of the documentation side of the bump. That was manual
before, and would have drifted silently.
