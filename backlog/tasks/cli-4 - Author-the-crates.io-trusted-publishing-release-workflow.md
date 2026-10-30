---
id: CLI-4
title: Author the crates.io trusted-publishing release workflow
status: In Progress
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Produce a reviewed `release.yml` that implements the two-crate trusted-publishing contract in decision-7.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The documented crates.io trusted-publishing flow, the GitHub environment `crates` binding and cargo's requirements are inspected, and the chosen action set is recorded in `release.yml`.
- [ ] #2 Release plan accounts for `release.yml`, environment `crates`, OIDC, macro-first order and partial-publish recovery.
- [x] #3 No live publish is required or performed for this task.
<!-- AC:END -->

## Notes

- 2026-10-30: retitled. The task previously framed this work as investigating an external release-workflow generator; the workflow is authored in this repository instead, so the task is now to write and review `release.yml`. ID and dependency edges are unchanged.

## Notes

- 2026-10-30: the maintainer confirmed the crates.io binding: trusted publishing is enabled for `clifacet` and `clifacet-macros` against `release.yml` with GitHub environment `crates`, matching decision-7. `.github/workflows/release.yml` now implements it: pinned `rust-lang/crates-io-auth-action` v1.0.6, job-scoped `id-token: write`, macro crate first, index polling before the main crate, and a publish step that skips any version already indexed so a partial release is recoverable by re-running.
- #2 stays open until a real dispatch runs: `workflow_dispatch` only fires from `main`, so the dry run (the default) is the first verification, and no live publish is part of this task.
