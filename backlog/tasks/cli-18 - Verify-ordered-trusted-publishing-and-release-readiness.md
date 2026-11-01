---
id: CLI-18
title: Verify ordered trusted publishing and release readiness
status: To Do
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-5
dependencies:
  - CLI-4
  - CLI-16
  - CLI-17
type: task
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rehearse the authored two-crate publication contract end to end.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `release.yml` publishes macro-first using environment `crates`, job-scoped OIDC and pinned official actions.
- [ ] #2 Macro-first publication, registry availability, partial-failure recovery and package contents are verified without an unauthorized live release.
- [ ] #3 Release is performed only after explicit maintainer authorization and recorded successful gates.
<!-- AC:END -->

## Notes

- 2026-10-30: two readiness gaps were found before this task's rehearsal and are now fixed in the
  release path itself: the first release could not date its changelog (`bump-version
  --promote-current` now does), and `release.yml` resolved the notes only after publishing (it now
  resolves them before anything is packaged, so a missing section fails with nothing to revert, and
  the tag step reuses the verified file). Both are recorded with their tests under CLI-20.
- Still unproven without a real run: AC#2's "registry availability" and "partial-failure recovery",
  which need a first real publish, and AC#3's authorization record. `--promote-current` must be run
  and merged before the rehearsal dispatch, because a dry run now checks the notes too.
- 2026-10-31: **AC#1 checked, statically.** `scripts/tests/test_release_workflow.py` now
  pins the properties a rehearsal would otherwise only discover by publishing something: a
  dispatch is the only trigger, the job runs in `crates`, `id-token: write` is scoped to
  the job rather than the workflow, the loop publishes `clifacet-macros` before
  `clifacet`, the token exchange is gated behind `!inputs.dry_run`, `dry_run` defaults to
  true, and every `uses:` is pinned to a 40-character commit SHA. Ten tests, all green.
- 2026-10-31: **AC#2 split into what can be proven offline and what cannot.** The
  package-contents half is proven: `scripts/check_packages.py` (task `check-packages`, in
  `test-local`, `quality` and CI) runs `cargo package --list` for both crates and checks
  contents and metadata. It found a real gap on its first run — neither crate packaged
  `LICENSE-MIT` or `LICENSE-APACHE`, because both live only at the repository root and
  cargo does not reach outside the package root — so the copies were added, with a
  byte-for-byte equality test against the root files so they cannot drift. Registry
  availability and partial-failure recovery still need a real publish, and the rehearsal
  checklist now says so in `docs/development.md` ("Rehearse a release").
- 2026-10-31: evidence for the skill loop. *TDD:* `scripts/tests/test_check_packages.py`
  (15 tests) came first and failed with `ModuleNotFoundError`; the second red round was the
  real tree, where the contract failed on the missing licence files, and the third was
  `cargo package` refusing a dirty tree, which `--allow-dirty` documents rather than hides.
  The workflow pins were written against an already-authored `release.yml`, so they went
  green on the first run: regression guards, not a behaviour change, recorded as such.
  *BDD:* narrowed and stated — no user-visible workflow, so a scenario would describe a
  maintenance check; the promise is covered by executable examples over file lists and
  manifests. *Clean code:* the contract is data (`REQUIRED_FILES`, `FORBIDDEN_PREFIXES`,
  `METADATA`) rather than a chain of conditionals, findings aggregate per crate so one run
  names everything wrong, and `cargo` is invoked through an injectable runner so the rules
  are tested without a build.
- Still open here: AC#2's registry availability and partial-failure recovery, and AC#3,
  which needs explicit maintainer authorization for the first real dispatch.
- 2026-11-01: the rehearsal was dispatched (run 41309561174) and failed in 24s at
  `Resolve the release notes`, with nothing packaged or published. Cause: `CHANGELOG.md` had no
  `## [0.1.0]` section. That is the guard behaving correctly — the missing section used to fail at
  the tag step, after both crates were live. The change was made (PR #29: `bump-version
  --promote-current`), and `bump_version.py --notes 0.1.0` now yields 196 lines.
- 2026-11-01: re-dispatch from the agent session returned `HTTP 403: Resource not accessible by
  integration` — `gh workflow run` is a third command the app token cannot perform, now recorded in
  the `AGENTS.md` operations table and in `docs/development.md`. Reading is unaffected
  (`gh run list`/`view`/`watch`, `gh pr checks`), so once a maintainer dispatches, the run can be
  watched from here to completion. AC#1's static checks and AC#2's package-contents half are done;
  the dry run itself and both registry facts are waiting on that dispatch.
