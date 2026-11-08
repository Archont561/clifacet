---
id: CLI-55
title: Keep the release's changelog commit from going stale
status: In Progress
assignee: []
created_date: '2026-11-08 11:15'
updated_date: '2026-11-08 12:30'
labels: []
milestone: m-12
dependencies: []
priority: high
type: bug
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every release leaves main failing `changelog-check`, and nothing notices.

`release.py regenerate_changelog` renders CHANGELOG.md while HEAD is the tag it just created, then commits and pushes it. That commit moves HEAD past the tag, and convco renders an `## [Unreleased]` section (with an `[Unreleased]: …/compare/vX...HEAD` link) whenever HEAD is past the latest tag, even when the commits since are hidden types. So the file the release commits is stale the moment it lands. Pushes made with the workflow's GITHUB_TOKEN do not trigger CI, so main is never checked; the next PR fails `Documentation checks` on a file it did not touch.

Observed in 0.3.1: run 42473663383 pushed 1b7a1c4 `chore(release): regenerate the changelog`, and PR #61 then failed "Check the changelog matches the commits". Reproduced in a worktree: rendered at `v0.3.1` the file has no `Unreleased`, rendered one commit later it gains the empty section and link. PR #61 committed the current render to unbreak main; this task removes the cause.

Proposed fix, to be confirmed test-first: render a stable `## [Unreleased]` heading (and its compare link) even when HEAD is exactly a tag, as Keep a Changelog does, so a render at the tag equals a render one hidden commit later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test in scripts/tests reproduces the bug first: a render at a tag differs from a render after one more hidden (`docs`/`chore`) commit, and it fails before the fix
- [ ] #2 After the fix those two renders are identical, so the commit `release changelog` makes cannot make CHANGELOG.md stale
- [ ] #3 The release notes are still exactly the new version's section: `release preflight` and `release tag` never include an empty `Unreleased` heading, covered by the existing release tests or a new one
- [ ] #4 The release step checks its own result: after committing, it runs the changelog check and fails the run if the file does not match, because CI does not run on the bot's push
- [ ] #5 docs/development.md §Release describes the stable heading and the self-check, and test-local is green
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Work log (TDD → clean-code; a repository-tooling bug, so no BDD scenario — the evidence is the scripts suite against the real convco):

- **Red first (AC#1).** `scripts/tests/test_changelog.py::test_a_render_at_the_tag_survives_the_commit_that_records_it` builds a real history (this repo's `.versionrc`, tags v0.1.0 and v0.1.1, HEAD on v0.1.1), renders through the production `generate(convco(..))`, adds one hidden commit and renders again. Both parameters (`chore(release): regenerate the changelog`, `docs: reword`) failed before the fix: the second render gained `## [Unreleased]` and its link. Reverting only `scripts/changelog.py` makes the suite fail again (import of `with_unreleased`).
- **Fix (AC#2).** `scripts/changelog.py` `with_unreleased` adds `## [Unreleased]` with the `vX...HEAD` compare link convco would write one commit later whenever convco left it out (HEAD exactly on a tag). It runs in `generate` only, so `render` stays a normaliser. The first release (no compare link to borrow) falls back to the configured repository: covered by `test_the_first_release_survives_the_commit_that_records_it`. Green: both renders identical.
- **Notes unchanged (AC#3).** The notes path (`notes`/`notes_for`, used by `release preflight`, whose file `release tag` attaches) does not go through `generate`, so it cannot gain the heading. `test_release_notes_never_carry_the_unreleased_heading` proves it with the real convco; the existing preflight/tag tests pass unchanged.
- **Self-check (AC#4).** Red first: `test_the_changelog_step_checks_the_file_against_the_commit_it_made` and `test_a_changelog_that_already_matched_is_still_checked` failed with "DID NOT RAISE", and the order test failed because no render followed the commit. `release.py regenerate_changelog` now renders again after `commit` and raises `ReleaseError` ("… is stale after the commit that recorded it … Run `pixi run changelog`") on a mismatch. The release.yml comment says so.
- **Real history.** In a worktree: before the fix a render at v0.3.1 differed from one at 1b7a1c4 (the bot's commit); after the fix they are identical. `changelog-check` on main still passes (HEAD is past the tag, so the committed file is unchanged).
- **Docs (AC#5).** docs/development.md §Release: the `release changelog` row, a paragraph on the stable heading and the self-check, and the release history corrected — it still said the push had never run; 0.3.1 ran it and left main stale.
- **Checks.** test-scripts 382 passed / 2 skipped, alint, actionlint, check-docs; test-local before push.
<!-- SECTION:NOTES:END -->
