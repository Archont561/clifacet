---
id: CLI-41
title: Lay out both examples one file per command
status: Done
assignee: []
created_date: '2026-11-07 21:46'
updated_date: '2026-11-08 01:56'
labels: []
milestone: m-7
dependencies: []
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested on 2026-11-07: remove examples/simple/examples and give examples/simple and examples/backlog-lite a nested, file-per-command layout instead of one large lib.rs. This applies decision-15's recommended convention (a file per leaf, registration by hand in src/commands/mod.rs) without the deferred clifacet::commands! macro (CLI-33). Behaviour-preserving refactor: no public path, test or output changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 examples/simple/examples is removed, and every doc that pointed at its two demos points at the tests that drive the same published API
- [x] #2 Each example keeps one file per command under src/commands/, registered by hand in src/commands/mod.rs (for backlog-lite, also the tree and its Request); lib.rs holds only the crate docs, the module list and re-exports
- [x] #3 No test file changes and the same 185 example tests pass; every previously public item is still reachable at the crate root
- [x] #4 Each example README's layout section describes the new structure
- [x] #5 The task records the refactor exception: no behaviour changes, so no new BDD scenario
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Refactor exception (AGENTS.md): no behaviour changes, so no new failing test or BDD scenario. The baseline was taken at public seams before the move and re-checked after it.

Evidence:
- Tests: the same 185 example tests before and after (`cargo nextest list` diffed: identical; 160 simple + 25 backlog-lite, all pass). `git diff` on `examples/*/tests` is empty, including the generated `documented_snippets.rs`.
- Output: both binaries were built from `main` in a temporary worktree and compared with the refactored ones over 40 invocations (23 `bl`, 17 `example`): help, version, usage errors, validation failures, `--dry-run` per leaf, confirmations and mode refusals. stdout, stderr and exit codes were identical for all 40.
- Public API: every item that was public at either crate root is re-exported from `lib.rs`; modules are private.
- Checks: clippy `-D warnings` and rustdoc `-D warnings --document-private-items` are clean for both crates.

Layout:
- backlog-lite: `src/commands/mod.rs` (root `Cli`, `Verb` tree, `Request`, `Action`), one file per leaf (list, show, new, start, done, search, doctor; each holds its flags, input, bridge/questions and forwarded argv), `commands/task.rs` (shared by show/start/done), `status.rs`, `invocation.rs`, `app.rs`. lib.rs: 51 lines, down from 709.
- simple: `src/commands/mod.rs` + `commands/example.rs`, `interaction.rs`, `exit.rs`, `entry.rs` (not `run.rs`: a module named `run` beside the function `run` makes rustdoc links ambiguous), `form.rs`. lib.rs: 64 lines, down from 668.
- `examples/simple/examples/{parity_report,tui_custom_screen}.rs` are removed. They duplicated `tests/harness_parity.rs` and `tests/tui_escape_hatches.rs`, which drive the same published API from outside the library crate; `docs/command-core.md` and `docs/tui-composition.md` now point at those tests. CLI-12 and CLI-13 still name the removed files in their closed notes; those are historical and were left as they are.

For CLI-33's re-measurement: both examples now follow decision-15's file-per-leaf convention by hand. What a `clifacet::commands!` macro could still save is exactly the `mod` lines and the `Verb` enum in `commands/mod.rs`, about 25 lines in backlog-lite.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed on maintainer authorisation (2026-11-08). All five ACs met in PR #59 (commits e4a23666, e240d0e0, adc9d0cc): both examples keep one file per command under `src/commands/`, registered by hand in `src/commands/mod.rs`; `lib.rs` holds only the crate docs, module list and re-exports; `examples/simple/examples` is gone and its docs point at the tests; no test file changed and the same 185 example tests pass; both READMEs describe the layout. Refactor exception recorded: no behaviour change, so no new scenario.
<!-- SECTION:FINAL_SUMMARY:END -->
