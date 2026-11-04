---
id: CLI-38
title: Prepare and publish 0.2.0
status: In Progress
assignee: []
created_date: '2026-11-01 22:46'
updated_date: '2026-11-04 13:52'
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
- [x] #1 The version is bumped through the runbook with `--relock`, and the lockfile change comes from the bot step rather than a hand edit.
- [x] #2 Every quality gate passes on the release commit before the tag exists.
- [x] #3 The changelog and the release notes are rendered from the commits; no section is written by hand.
- [x] #4 Both crates publish from one authorized dispatch through the `crates` environment, in order, with the tag matching the published version.
- [ ] #5 The remaining acceptance criterion of CLI-20 is satisfied here: the relock path is exercised from a `main` dispatch, and the evidence is recorded on that card.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-04: **0.2.0 is published.** Release run [41841115583](https://github.com/Archont561/clifacet/actions/runs/41841115583) was dispatched by the maintainer from `main` at `32084490` with `dry_run` unchecked, and every step succeeded: preflight, package, the OIDC exchange, the publish loop (macro crate, then main), the tag, and the changelog step. Tag `v0.2.0` points at `32084490`. The GitHub Release `v0.2.0` is published, not a draft, and its notes are rendered from the commits. The publish loop polls the index and fails if a version never appears, so a green step means both `0.2.0` crates were indexed. I could not query crates.io directly from the sandbox, so that is the evidence.
- **AC#1.** The bump was `pixi run bump-version 0.2.0 --relock` (PR #45, commit `85bde202`): three version sites, plus the four workspace crates' entries in `Cargo.lock`, with no dependency movement. The lock change came from `--relock`, not a hand edit. CI's `lock guard` passed, and the `relock` repair job skipped because there was nothing to repair. I read 'the bot step' as 'the automated relock rather than a hand edit'; if it meant the `relock.yml` repair job must produce the change, this is not met.
- **AC#2.** CI run [41839976834](https://github.com/Archont561/clifacet/actions/runs/41839976834) on `32084490` passed every job before the release dispatch, as did the sandbox publish run 41839976685.
- **AC#3.** The release notes are rendered by the workflow from the commits. The committed `[0.2.0]` section is rendered by `convco` (`pixi run changelog`), not written by hand, but it was rendered locally rather than by the release step. See the next note.
- **AC#4.** One authorised dispatch through the `crates` environment published `clifacet-macros`, then `clifacet`, then tagged `v0.2.0`, matching the recorded version.
- **AC#5 stays open.** CLI-20 AC#4 also requires the post-tag changelog commit to land, and it did not. `release changelog` fetched the tag and went straight to `commit`, which only commits a changed file. It never rendered, so it reported 'already matches the tag' and pushed nothing. This is the step's first real run, because the changelog tooling arrived after 0.1.0. Fixed test-first in `scripts/release.py`: a new `regenerate_changelog` does fetch → render → write → commit, with four tests in `test_release.py`, and a `ChangelogError` is now a message instead of a traceback. It can only be proven by the next release.
- **Found on the way.** In PR #45, a `bump_version` test rewrote the real manifests back to 0.1.0 after the bump. It now runs on fixtures.
<!-- SECTION:NOTES:END -->
