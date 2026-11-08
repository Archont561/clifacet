---
id: CLI-55
title: Keep the release's changelog commit from going stale
status: To Do
assignee: []
created_date: '2026-11-08 11:15'
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
