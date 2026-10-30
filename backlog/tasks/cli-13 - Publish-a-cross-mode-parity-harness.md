---
id: CLI-13
title: Publish a cross-mode parity harness
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-4
dependencies:
  - CLI-10
  - CLI-12
type: task
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make the scripted test facilities available as a documented consumer-facing API.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A representative command runs through flag, scripted prompt and headless TUI inputs.
- [x] #2 Assertions cover equivalent typed input, final output, failures and recorded effects.
- [x] #3 The harness uses only public supported extension points and is usable by an external example.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `clifacet::harness` — `Outcome` (one run reduced to the four subjects two
  runs can disagree about), `Recording` (a `Command` wrapper that records the input each execution
  received, plus an `Effects` handle), and `Parity` (a comparison that names the mode and the subject
  that disagreed). Always compiled: it has no dependency of its own, so a `testing` feature would only
  add a way to get the boundary wrong.
- AC#1: `tests/harness_parity.rs` and `tests/features/parity.feature` run the example command through
  flags (`main_with_command`), a scripted prompt (`Capabilities::terminal()` + `Scripted`) and the
  headless form (`run_tui_with` + `Key`s), and `examples/parity_report.rs` runs the same three and
  prints the table.
- AC#2: the four assertion kinds each have a case — typed input (the recorded `ExampleInput` equal
  across modes), output (`data`), failures (a validation refusal keeps the flags' exact words and
  records nothing; an execution failure keeps its exit code across modes; a cancellation is compared
  on the contract) and recorded effects (`Recording` is the only way effects are produced, and an
  empty record is asserted to mean "the command never ran").
- AC#3: `examples/parity_report.rs` is outside the crate, so it can reach only the published surface —
  if the harness needed an internal hook the example would not compile. Everything the tests use
  (`main_with_command`, `run_with_command`, `tui_submit_with`, `run_tui_with`) is public and delegated
  from the existing entry points, which are unchanged.
- BDD: `tests/features/parity.feature` with `tests/parity_bdd.rs` — one scenario compares all three
  modes on data, exit code and recorded input; the other asserts that a rejected answer reaches the
  command in neither mode and fails identically.
- TDD evidence: `tests/harness.rs`, `tests/harness_parity.rs`, `tests/parity_bdd.rs` and the feature
  file were written first and failed to compile against a nonexistent `clifacet::harness` and the
  four missing entry points; the implementation then grew until they passed. Two test-side mistakes
  were fixed by correcting the test: the execution-failure case compared a prompt run that answered
  `ada` against a flag run with no `--name` (so the inputs differed for a reason unrelated to the
  mode), and the refusal case looked for the validation message on stderr when the form keeps it on
  the form by design.
- Design decisions: the harness measures and compares, and never runs an application's frontends —
  each mode is the application's own entry point, so a comparison covers the real `clap`/policy/form
  path instead of a test-only runner that could drift. There is no mode enum or runner trait, because
  describing frontends to the library is the global registration decision-4 forbids. `Recording`
  records *before* executing, so a failure still records what reached the command. `Parity::comparing`
  exists because frontends word a cancellation differently on purpose; the contract is compared, not
  the prose. `Parity` holds its reference separately from the other runs, so "no runs to compare" is
  not a state that has to be handled.
- Coverage: `cargo llvm-cov` reports 1026/1031 source lines. The five uncovered lines are the
  process's own terminal in `tui::app::run_on_with`, unchanged from CLI-11/12.
- Deliberately out of scope: secret provenance (CLI-14) and the stream contract work (CLI-15) — the
  harness can compare whatever those tasks add, but does not decide it.
