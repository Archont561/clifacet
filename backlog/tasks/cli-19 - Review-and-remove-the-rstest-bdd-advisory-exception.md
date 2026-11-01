---
id: CLI-19
title: Review and remove the rstest-bdd advisory exception
status: Done
assignee: []
created_date: '2026-10-29 15:46'
labels: []
milestone: m-4
dependencies: []
priority: medium
type: task
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Track the temporary RUSTSEC-2026-0374 exception in cargo-deny and cargo-audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Review whether rstest-bdd or its transitive dependencies can avoid the unmaintained basic-toml crate.
- [x] #2 Remove both ID-wide exceptions when possible; otherwise document the current dependency path and review outcome.
<!-- AC:END -->

## Notes

- 2026-10-30: assigned to m-4 (parity and safety), which carries the dependency and safety guarantees; the task had no milestone. Reassign if the maintainer prefers m-5. The exception itself is unchanged: `.cargo/audit.toml` still ignores RUSTSEC-2026-0374 until #1 is reviewed.
- 2026-10-31: **#1 reviewed — no, it cannot be avoided on `rstest-bdd` 0.6.0.** The
  vendored manifest of that crate lists `i18n-embed` as a plain non-optional dependency,
  its features are `diagnostics`, `mutable_world_macro` and `test-support`, and the
  lockfile chain is `rstest-bdd → i18n-embed → i18n-embed-impl → i18n-config →
  basic-toml`. No feature selection removes it, and a `[patch.crates-io]` fork of a test
  dependency would cost more than the advisory it hides. The crate is a dev-dependency,
  so nothing `basic-toml` parses ships in a published crate — but the exception stays by
  advisory ID, because that is not a property the exception can enforce.
- 2026-10-31: **#2 taken as the documented branch**, and made verifiable rather than
  prose. `docs/development.md` now quotes the path verbatim and records the outcome;
  `scripts/check_advisory_exception.py` re-renders that path from `Cargo.lock` and fails
  with `stale-exception`, `missing-exception` or `documented-path` when this review stops
  being true. Wired into `test-local`, `quality` and a CI step, so the expiry is checked
  on every push. Verified here: the first run reported `documented-path` against the path
  the guide assumed, because the review had missed `i18n-embed-impl 0.8.4` in the chain —
  the path above is what the lockfile actually says.
- 2026-10-31: evidence for the skill loop. *TDD:* the failing test came first —
  `scripts/tests/test_advisory_exception.py` was written before the script (red with
  `ModuleNotFoundError`), then the tree-consistency test failed against the real
  repository and drove the parser fix and the corrected path; a second red round covered
  `main` (`TypeError: unexpected keyword argument 'root'`) before the entry point was
  given a `root` parameter. 19 tests, all green. *BDD:* narrowed and stated — this
  change has no user-visible workflow, so an `rstest-bdd` scenario would describe a
  repository check rather than a promise to a user; the promise is expressed as
  executable examples over the four files the check reads. *Clean code:* one source of
  truth for the path (the lockfile; the guide quotes what the script renders), the
  path constants reused instead of literal filenames repeated, findings that name both
  the problem and the action, and no TOML dependency added for a three-key file that a
  line reader handles.
- 2026-11-01: the checker caught the agent. Adding "Actions → release → Run workflow" to
  `docs/development.md` made `documented-path` fire: `documented_path()` returned the first
  line anywhere in the file that contained the separator, so a prose arrow passed for a
  dependency chain and the check reported its own documentation. Fixed by scoping the
  search to the `## Dependency advisory exception` section and requiring every hop to be a
  `name version` pair, which is what distinguishes a path from a sentence. Three tests
  added first (red): a prose arrow before the real chain, a chain that lives in the
  section while another section also uses arrows, and a document without the section.
  The test fixture's own lock was stale in the same way the guide had been — it lacked
  `i18n-embed-impl` — so the fixture now mirrors the real graph and would have failed on
  the original omission.
