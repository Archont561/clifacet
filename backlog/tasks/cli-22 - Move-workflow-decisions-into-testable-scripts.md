---
id: CLI-22
title: Move workflow decisions into testable scripts
status: Done
assignee: []
created_date: '2026-11-01 10:00'
labels: []
milestone: m-5
dependencies:
  - CLI-18
  - CLI-20
type: task
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release and CI workflows carried decisions in shell: a range `if`/`elif`/`else`, a
`sed` loop over crate manifests, a publish loop polling the registry index, and a summary
built by hand. None of it could be exercised without a dispatch, and the answers that
matter most — what happens when a crate is already indexed, when the index cannot be
read, when the version never appears — arrive once, on a real release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every decision in `ci.yml` and `release.yml` lives in a Python script under `scripts/`, with pytest coverage that needs neither a registry nor a runner.
- [x] #2 Each workflow step is a single `pixi run <task>` call, and a test fails when a `run:` block grows shell control flow.
- [x] #3 The generated sandbox workflows (`publish-sandbox.yml`, `relock.yml`) and `scripts/restore.sh` are untouched.
<!-- AC:END -->

## Notes

- 2026-11-01: four scripts, each with a test module of the same name.
  `scripts/github_actions.py` holds the two GitHub file protocols (`GITHUB_OUTPUT`,
  `GITHUB_STEP_SUMMARY`, `::error::`) so a script that replaces a step can still talk to the
  next step, and prints instead of silently dropping the value when run locally.
  `scripts/commit_range.py` resolves the commit range from the event (`pull_request` →
  base..HEAD, `push` → before..HEAD, all-zeros before → the tip plus a window) and makes the
  single quoted `convco` call. `scripts/advisory_cache.py` computes the ISO week key — the
  week-year is not the calendar year, which is the edge worth a test — and resets restored
  clones. `scripts/release.py` owns `preflight`, `package`, `report`, `publish` and `tag`.
- The effects are injected rather than mocked: `fetch(url) -> (status, body)` for the sparse
  index and `run(argv) -> exit code` for cargo, git and gh. So the publish loop is asserted
  against a fake index — 404 means publish, 500 means stop rather than publish blind, a
  version that never appears fails, and a crate that is already indexed is skipped so a
  partial release is finished by re-running the same command.
- 2026-11-01: `release.yml` went from 216 lines to 90, and now names no crate, no index URL
  and no `cargo publish`; a test asserts all three absences, because a copy in YAML is a
  second source of truth. `ci.yml` lost its range `if`/`elif`, its `for`/`git reset` loop and
  two `$GITHUB_OUTPUT` echoes. `prove_msrv.py` gained `--github-output NAME` so the MSRV job
  asks for its output instead of piping through `echo`.
- 2026-11-01: the invariant is a test, not a convention.
  `scripts/tests/test_release_workflow.py` extracts every `run:` block, including block
  scalars, and fails on `if`, `for`, `while`, `case`, `$(`, `&&`, `||`, a pipe or a redirect.
  It also checks that every `run:` line starts with `pixi`, `cargo`, `rustc`, `rustup` or
  `python3`, and that a new
  `scripts/tests/test_pixi_tasks.py` case proves every task a workflow calls exists in
  `pixi.toml`.
- TDD evidence. Red states, in order: `ModuleNotFoundError: scripts.github_actions`,
  `scripts.commit_range`, `scripts.advisory_cache`, `scripts.release`, and
  `ImportError: crate_pins`. Then real defects the tests caught rather than the reverse: the
  `run` seam returned an exit code while `_output` expected stdout, so the "published from
  <sha>" line was re-sourced from `GITHUB_SHA`; and two tag tests conflated "no release yet"
  with "the create failed" by returning one exit code for every command. One test was
  rewritten because it asserted the wrong contract (`append_summary` ends each section with
  a blank line) and renaming it to say so was correct, not a weakening.
- The two bugs that only running found, both now covered by
  `scripts/tests/test_script_entry_points.py`: `python scripts/release.py` puts `scripts/` on
  `sys.path` rather than the repository root, so the `scripts.*` imports raised
  `ModuleNotFoundError`; and after adding the path bootstrap, an unused-in-import-mode
  `import sys` was missing from `advisory_cache.py`, which was a `NameError` at the first
  task run. The suite imported the modules as packages and saw neither, which is exactly why
  the entry-point test starts each script in a subprocess.
- BDD: narrowed and stated. This change adds no user-visible workflow — it moves maintenance
  logic — so an `rstest-bdd` scenario would describe a repository check rather than a promise
  to a user. The promise is expressed as executable examples over the two seams.
- Clean code: one entry point per concern, one source of truth per constant (crate names,
  index prefix, publish order live only in `scripts/release.py`), findings that name the crate
  and the action, and presentation (`dry_run_summary`) separated from execution (`publish`).
- Untouched on purpose: `publish-sandbox.yml`, `relock.yml` and `scripts/restore.sh` are
  generated by `pixi-sandbox init` and `sandbox-check` verifies them; editing them by hand
  would fail that gate.
