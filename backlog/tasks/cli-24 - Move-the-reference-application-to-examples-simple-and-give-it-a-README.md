---
id: CLI-24
title: Move the reference application to examples/simple and give it a README
status: Done
assignee: []
created_date: '2026-11-01 17:22'
updated_date: '2026-11-01 17:26'
labels: []
milestone: m-5
dependencies: []
type: task
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The example is the first thing a reader runs, and it lived under crates/ beside the two published crates with no README of its own. Move it to examples/simple so the layout says what it is, and present it there: sources, tests and a README that covers running it, its flags, its modes, its streams, its exit codes and which test proves each promise. Mechanical: no behavior changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The crate lives in examples/simple, its path dependency resolves, and the workspace member list, Alint rules, hook globs and check-docs paths follow it.
- [x] #2 docs/, README.md and CHANGELOG.md name the new location; no guide points at crates/clifacet-example.
- [x] #3 examples/simple/README.md presents the basic CLI: how to run it, its flags, the four modes, the streams and exit codes, the layout and the tests.
- [x] #4 The package name stays clifacet-example, so cargo run -p clifacet-example and every documented command keep working.
- [x] #5 test-local is green, including check-docs snippet staleness and the script tests that read the snippet path.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Moved with `git mv` so the history follows the files: `crates/clifacet-example` is now `examples/simple`, and the crate keeps its package name (`clifacet-example`) so `cargo run -p clifacet-example` and every documented command keep working. The path dependency became `../../crates/clifacet`.

- Followed the move: workspace members (`examples/simple` last, matching the Alint `toml_path_equals` list, which was reordered with it), the two `file_exists` Alint rules (`require-example-command`, `require-example-manifest`), the `check-docs` hook glob, `scripts/check_docs.py` (`SNIPPETS`, `SOURCE_ROOTS`, `check_staleness`) and the fixture path in `scripts/tests/test_check_docs.py`.
- Every `crates/clifacet-example` reference in `README.md`, `docs/*.md` and the changelog became `examples/simple`; anchors were re-derived by `check-docs`, which also regenerated `documented_snippets.rs` at its new path.
- `examples/simple/README.md` is new: how to run it (including a piped run), the flag table, the four modes, the streams and exit codes, the layout of `src`/`tests`/`examples`, and a table of which test proves which promise. `docs/README.md` and `docs/getting-started.md` link to it, and the root README names it beside the workspace table.
- TDD/BDD exception, stated rather than assumed: this is a move plus a page of prose, so there is no failing test to write first and no new user-visible workflow. The evidence that nothing changed is the suite: `test-local` is green, and the 28 tests in `examples/simple` plus the library's own are the same tests at a new path.
- Not done, and deliberately: the generated `relock.yml` and `publish-sandbox.yml` still watch `crates/**/Cargo.toml` only, so a change under `examples/` does not trigger a relock. Those files are rendered by `pixi-sandbox init`; editing them by hand would fail `sandbox-check`, so the fix belongs in the sandbox policy, not here.
- A second example (`examples/advanced`) is not created. The directory exists to make room for one.
<!-- SECTION:NOTES:END -->
