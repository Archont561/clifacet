---
id: CLI-20
title: Automate the version bump and release runbook
status: In Progress
assignee: []
created_date: '2026-10-30 16:30'
updated_date: '2026-11-01 22:46'
labels: []
milestone: m-10
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
- [x] #2 The bump leaves `CHANGELOG.md` alone: it is rendered from the commits, and a release whose commits say nothing is refused before anything is packaged.
- [x] #3 `release.yml` tags the release and attaches the rendered notes only after both crates publish.
- [ ] #4 A full release is rehearsed end to end on a real version bump.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-01: the full release happened end to end (run [41344566334](https://github.com/Archont561/clifacet/actions/runs/41344566334), tag `v0.1.0`, GitHub Release `v0.1.0`). That proves everything AC#4 was written for — the bump sites agreeing, the notes resolving before anything was packaged, both publishes in order, and the tag and release created after them — except one clause: it was **not on a real version bump**. `0.1.0` was dated with `--promote-current`, which deliberately leaves the version sites alone.
- AC#4 therefore stays open, narrowed to what it now means: the next release must exercise `bump-version 0.1.1 --relock` — manifest sites, changelog promotion, lock refresh — and a dispatch from the resulting `main`. That is one release away and needs no new tooling.
- Consequence to know before that release: `CHANGELOG.md`'s `## [Unreleased]` section is now **empty**, because the entries merged between the promotion and the release shipped inside `0.1.0` and were moved into its section rather than left describing a version they were already part of. The bump refuses an empty section, which is the intended guard: the next release has to describe itself.
- Also reconciled: the `v0.1.0` release notes on GitHub were generated from the promotion, so they do not list the entries that were moved afterwards. Refreshing them is `gh release edit v0.1.0 --notes-file <file>`, a one-line maintainer action; it was not done here because the notes are a published artifact.

- 2026-11-01: `v0.1.0`'s published notes were refreshed to the corrected changelog section (`gh release edit v0.1.0 --notes-file …`). The gap was real and structural, not a slip: `release.py tag` deliberately leaves an existing release alone, so the manual step is the only way to rewrite notes after the fact. `docs/development.md` §Release now says so, and says why re-dispatching cannot do it.

- 2026-11-01: AC#2 rewritten, with the ACs re-set rather than silently re-worded: it used to require that "the changelog is promoted by the same command, and an empty `Unreleased` section blocks the bump". Both halves are gone because the changelog is no longer written by hand or promoted by the bump — `convco` renders it from the commits and the release refuses a tree whose commits since the last tag say nothing, which is the same protection sourced from the commits instead of the file. The evidence for the new wording is the test pair that now guards the absence: `test_main_does_not_touch_the_changelog` in `scripts/tests/test_bump_version.py`, and `test_preflight_ignores_a_committed_changelog` in `scripts/tests/test_release.py`. `--promote-current` and `--skip-changelog` are deleted, not deprecated, so a stale runbook step fails loudly.
- AC#4 is untouched and still open for the same reason: the next release has to be cut on a real `bump-version 0.1.1 --relock`, and now also has to prove the post-tag changelog commit lands.
<!-- SECTION:NOTES:END -->

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
- 2026-11-01: the first dry-run dispatch was attempted and **failed where it was designed
  to fail** (run 41309561174, 24s, nothing packaged or published): `Resolve the release notes`
  stopped the job because `CHANGELOG.md` had no `## [0.1.0]` section. That is the guard from
  2026-10-30 doing its job — the failure arrived before the first publish instead of at the tag
  step — and it confirms the rehearsal cannot pass until the changelog is promoted.
  `pixi run bump-version --promote-current` was then run: `Unreleased` became
  `## [0.1.0] - 2026-11-01`, an empty `Unreleased` section was reopened above it,
  `bump_version.py --notes 0.1.0` now emits 196 lines of notes, and `--check` still reports
  that all three version sites record `0.1.0`. The manifests were not touched, as designed.
- 2026-11-01: promoting the changelog broke five tests in `scripts/tests/test_bump_version.py`,
  which had encoded "the repository is in pre-release state" by reading the committed changelog.
  Fixed by making the fixtures a function (`_install_fixtures`) that writes the module's own
  `CARGO`/`PIXI`/`CHANGELOG` texts into `tmp_path`, so `main()` is exercised against a known
  changelog rather than against whatever the calendar made the real one. Exactly one test still
  reads the committed tree — `test_main_check_passes_on_the_committed_manifests` — because
  "the three version sites agree" is true both before and after a release. One test was added
  (`test_main_lists_the_sections_it_did_find`) so the `found:` message is covered by a fixture
  rather than by a real changelog that happened to lack a version. 190 Python tests pass.
- TDD evidence for that fix: the red state was produced by a real change (the promotion), not by
  a contrived assertion — five failures, all `AssertionError` on exit codes, and the fix was
  applied to the tests because the *product* behavior was correct and desirable.
- #4 remains open: the rehearsal is re-dispatched after this merges, and it proves ordering,
  version sites, notes and package contents. A full release — the publish itself — is AC#3 of
  CLI-18 and still needs explicit maintainer authorization.
