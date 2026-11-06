---
id: CLI-40
title: 'CI: a documentation lane for docs-only changes'
status: Done
assignee: []
created_date: '2026-11-06 18:13'
updated_date: '2026-11-06 18:47'
labels: []
dependencies: []
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A change that touches only documentation runs a light lane of the checks that documentation can break, instead of the full Rust matrix (quality, coverage, MSRV and three platforms). Everything else runs all of CI. Requested by the maintainer on 2026-11-06, who chose one workflow with a classifier job over two path-filtered workflows (one tested path list; skipped jobs satisfy required checks), and the broad definition of documentation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A tested script classifies a change as documentation only when every changed path is under backlog/ or docs/, is a Markdown file, or is a root LICENSE file; any other path, an empty change, a dispatch, a branch's first push or a diff git cannot compute runs all of CI.
- [x] #2 On a documentation-only change, the quality, coverage, MSRV and platform jobs are skipped and a documentation job runs alint, the skills, Backlog, guide and changelog checks; the commit-message job runs on every change.
- [x] #3 If the classifier fails, the full jobs still run: a broken classifier never turns CI green by skipping it.
- [x] #4 A test fails if a crate or example starts compiling a Markdown file in (include_str!/include_bytes!), since that would make a Markdown edit a code change the classifier calls documentation.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Decisions

- **One workflow with a classifier job, not two path-filtered workflows** (maintainer's choice, 2026-11-06). The path rule lives once, in `scripts/change_scope.py`, and is unit-tested; GitHub's `paths:`/`paths-ignore:` globs cannot be. Skipped jobs count as passing for a required check, whereas a path-skipped workflow leaves a required check pending forever. A mixed change does not run the documentation checks twice.
- **Broad definition of documentation** (maintainer's choice): `backlog/**`, `docs/**`, any `*.md`, and the root `LICENSE-*` files. A license inside a crate is code, because the crate publishes it.
- **The range is `commit_range.resolve_range`**, so the classifier and the commit-message check mean the same thing by "this change". `git diff --name-only -z --no-renames <range>`: `-z` because Backlog file names have spaces, and `--no-renames` so moving `src/x.rs` to `docs/x.md` lists the deleted `.rs` too.
- **Fail safe everywhere.** No range (dispatch, first push), an empty diff, a git failure or a missing git all output `docs-only=false`, and the script never exits non-zero. The full jobs are gated with `!cancelled() && needs.changes.outputs.docs-only != 'true'`, so a classifier that crashes leaves the output empty and the full jobs run.
- **The documentation job runs a subset of `quality`**: alint, skills-check, backlog-check, check-docs and changelog-check, with full history for the changelog. A test asserts each is also in `quality`.
- **`python3`, not Pixi, for the classifier**, as the MSRV job does: it is standard library only, and the light lane should not set an environment up twice. It still has a `change-scope` Pixi task, per `test_pixi_tasks.py`'s one-task-per-workflow-script rule. That edits `pixi.toml`, so merging this runs `publish-sandbox` once on main.
- `relock.yml` dispatches `ci.yml` with `workflow_dispatch`, which always takes the full lane, and it reads only the run's overall conclusion, so it is unaffected.

## Evidence

- Red first: `scripts/tests/test_change_scope.py` failed at collection with `ModuleNotFoundError: No module named 'scripts.change_scope'`. With the script in place, `main` failed with `SystemExit: 2` because the tests passed no argv and argparse read pytest's; the tests now pass `[]`, as `commit_range`'s do. The 11 wiring tests stayed red until `ci.yml` was edited.
- Now 50 tests in `test_change_scope.py`; `test-scripts` 371 passed, 2 skipped; actionlint, alint, check-docs and sandbox-check are clean.
- Against real history: PR #53's range (backlog only) gives `docs-only=true`, PR #52's gives `false` (naming `crates/clifacet/src/command.rs`), and a dispatch gives `false` with "no base commit to compare against".
- BDD exception: a CI change with no user-visible workflow in the library (AGENTS.md's mechanical-change narrowing); the behaviour is specified by the classifier and wiring tests instead.

## Open

- AC#2 is wired and statically tested but not yet seen live: this PR changes code, so it takes the full lane. The first documentation-only pull request after the merge (closing this task is one) will be the live proof.

Closed by the maintainer on 2026-11-06 after PR #54 merged (260a1422; main CI took the full lane, green). This closing change touches only this file, so its own CI run is the live test of AC#2; the run is recorded below once seen.

AC#2 seen live: PR #55's CI run 42273752814 ran Classify the change, Documentation checks and Conventional commit messages (all success) and skipped Repository and Rust quality, Tests and source coverage, Minimum supported Rust version and Supported platforms — about a minute, against about six for the full lane.
<!-- SECTION:NOTES:END -->
