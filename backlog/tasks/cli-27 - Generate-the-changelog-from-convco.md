---
id: CLI-27
title: Generate the changelog from convco
status: Done
assignee: []
created_date: '2026-11-01 20:38'
updated_date: '2026-11-01 20:42'
labels: []
milestone: m-5
dependencies: []
type: task
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CHANGELOG.md is hand-maintained while every other input to a release is derived: the version sites are rewritten by one script, the notes are verified before packaging, and the commit messages are already checked by convco. A hand-written section can drift from the commits it claims to describe, and did — entries merged before the 0.1.0 release shipped in the tarball while sitting under `Unreleased`.

Make the commit history the single source of the changelog: `convco changelog` renders it, a tested script owns the file, CI fails when the committed copy is stale, and the release workflow regenerates it after tagging.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `.versionrc` decides what the changelog contains: user-facing sections only, Keep a Changelog headings, and no false issue links from prose.
- [x] #2 `scripts/changelog.py` renders `CHANGELOG.md` from `convco changelog`, with pytest coverage, `--check`, `--print` and `--notes VERSION`.
- [x] #3 `release.py preflight` derives the notes from the commits since the last tag, and refuses a release with nothing to say.
- [x] #4 `bump_version.py` no longer edits `CHANGELOG.md`; the version sites and `--check` stay.
- [x] #5 `release.yml` regenerates and pushes the changelog after tagging, and CI fails when the committed file is stale.
- [x] #6 The guides describe the new flow, and every gate still passes.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- 2026-11-01: built, with the renderer's tests written first: `scripts/tests/test_changelog.py` failed on `ModuleNotFoundError: No module named 'scripts.changelog'`, then on the placeholder-free expectations, then passed 15/15. The red run was also where the first real bug surfaced — `^#+\s+` inside a `re.VERBOSE` pattern is a comment, so no version heading matched anything, and every test that asserted a heading failed rather than the file quietly rendering as prose.
- `.versionrc` is now the policy: `docs`, `build` and `ci` are `hidden: true` alongside `test`, `chore` and `style`, so the changelog carries what a user upgrading would act on; the generated `0.1.0` section is 86 lines instead of 176. `issuePrefixes` is `["closes #", "fixes #"]` rather than `["#"]`, because the bare prefix read the `AC#1`/`AC#2`/`AC#3` in an earlier commit body as `closes #1 #2 #3` and linked four issues that the commit never mentioned. Verified both ways against the real history: with `#` the false links appear, with the intent prefixes they do not.
- Rendering decisions, each with a test: a patch version one heading level deeper than a minor one becomes `##` (that is convco's default template, not something a reader should have to notice); headings become `## [0.1.0] - 2026-11-01`; the compare URL moves from the heading to a reference at the bottom, where Keep a Changelog keeps it; blank-line runs collapse and every heading gets one blank line above it, so re-rendering is byte-stable and `check` means something.
- `Unreleased` carries neither a date nor a commit SHA: convco writes both, and either would rewrite the file on every commit, whether or not a user-visible section changed. Its link is `compare/<last tag>...HEAD`.
- `--notes VERSION` is the same renderer on convco's unreleased section, so `release.py preflight` now derives the notes from the commits instead of reading a section a human wrote; `UnreleasedError` replaces the old empty-section guard and says "nothing to release: no conventional commit follows the last tag". A release whose commits say nothing stops before anything is packaged, as before — just sourced from the commits rather than the file.
- `bump_version.py` lost `promote_changelog`, `_update_links`, `changelog_notes`, `_sections`, `--promote-current`, `--notes` and `--skip-changelog`; it keeps the three version sites, `--check`, `--dry-run` and `--relock`. Two of its tests are now about the absence: `test_main_does_not_touch_the_changelog` and a `--notes`-free CLI. `test_release.py` gained `test_preflight_ignores_a_committed_changelog`, which writes stale prose into the file and asserts it cannot reach the notes.
- `release.yml` regenerates and pushes the changelog after the tag step (fetch-depth 0, tag re-fetched by `release changelog`, token from the environment because the checkout does not persist one), and `ci.yml`'s `quality` job runs `changelog-check` with full history. `test_release_workflow.py` pins the ordering: the regenerate step sits after the tag step, is gated on `!inputs.dry_run`, and CI's quality job checks out with `fetch-depth: 0`.
- Shallow clones are refused by name rather than rendered empty: `_ensure_history` reads `git rev-parse --is-shallow-repository` and tells the caller to fetch or check out deeper. That is what this sandbox did on the first attempt, and an empty changelog written over a good one is the failure that would have cost the most.
- Gates: `test-scripts` 305 passed / 2 skipped, `check-docs` 10 pages, `alint` 46 rules, `actionlint`, `backlog doctor`, `changelog-check`.
<!-- SECTION:NOTES:END -->
