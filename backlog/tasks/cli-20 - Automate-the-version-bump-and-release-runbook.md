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
- 2026-10-30: two gaps in the release path closed, both found while answering "when can
  0.1.0 ship". (a) The first release had no supported way to date the changelog: every bump refused a
  version the manifests already record, and `promote_changelog` refuses a version that already has a
  section, so `## [0.1.0]` could not be created and `bump_version.py --notes 0.1.0` failed. Added
  `--promote-current`, which promotes `Unreleased` at the recorded version and leaves the manifests
  alone, with `--dry-run`. (b) `release.yml` read the changelog only at the tag step, after both
  crates were published, so a missing section turned a successful publish into a failed release. The
  job now resolves the notes before anything is packaged or published (dry runs included, which is
  how the gap is found without publishing) and the tag step reuses that verified file.
- TDD evidence, scripts: the five `--promote-current` tests and `scripts/tests/test_release_workflow.py`
  were written first and failed for the right reasons — `--promote-current` was an unrecognised
  argument (argparse exit 2), and the workflow assertions failed on `substring not found` and on
  `bump_version.py --notes` still appearing in the tag step. One test-side mistake was fixed by
  correcting the test: "Unreleased is empty" was written as a link definition rather than a heading,
  so the script correctly reported a missing section instead of an empty one.
- The workflow test is deliberately text-based rather than YAML-parsed: the repository has no YAML
  dependency, and the two properties worth locking are ordering facts (`Resolve the release notes`
  precedes the first `cargo publish`, and the tag step names the resolved file instead of re-reading
  the changelog).
- Still open: #4 (a full rehearsal on a real version bump). With `--promote-current` in place that
  rehearsal is now `--promote-current` → dry-run dispatch → `cargo publish --dry-run`-equivalent
  checks, and it stays blocked on maintainer authorization rather than on tooling.
