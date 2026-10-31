---
id: CLI-15
title: Complete output and exit behavior contracts
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-4
dependencies:
  - CLI-13
type: task
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Finalize text/machine streams and app-owned exit mapping across modes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Machine-readable stdout contains only intended data and does not prompt or emit ANSI under the chosen policy.
- [x] #2 Success, validation failure, cancellation and execution failure are distinct under a documented app policy.
- [x] #3 BDD and parity tests cover representative outcomes.
<!-- AC:END -->

## Notes

- 2026-10-31: the stream half of the contract was already mostly true — prompts go to stderr, diagnostics
  to stderr — but two things were not. A full-screen run drew its screen on `stdout`, so a TUI put
  escape sequences into the data stream; and it reached for `std::io::stderr()` rather than the
  diagnostic stream the application passed, which made it untestable and bypassed the caller. The form
  now draws on the caller's diagnostic stream: `run_tui_on` hands `err` to `tui::run_on` and then
  reports to the same writer, and `finish_tui` is the one place a form's outcome becomes an exit code,
  so the two entry points cannot drift. `Capabilities::detected` follows the same rule — a TUI needs a
  terminal for the screen (`stderr`) *and* for the data (`stdout`), because a piped data stream is
  asking for data, not for a screen.
- AC#1: `tests/outcomes.rs` asserts `stdout` is exactly the greeting in flag, prompt and form modes,
  contains no escape sequence, and that a question reaches only the prompt's own stream; `--tui` is
  driven through `main_with` and asserted to leave `stdout` empty while the alternate screen appears on
  the diagnostic stream. `tests/harness_parity.rs` adds the cross-mode version: no mode writes a
  screen or a question to the data stream, and all three agree on exit code and data.
- AC#2: `clifacet::contract::ExitPolicy` makes the mapping a policy instead of four scattered `match`
  arms; `ExitPolicy::standard()` is 0 / 2 / 2 / 130 / 1. `example::EXIT` is that policy and the
  existing constants are now names for it, so the two cannot drift. `tests/outcomes.rs` asserts the
  four outcomes a caller distinguishes are pairwise distinct, that usage deliberately shares
  validation's code (asserted, so the overlap reads as a decision), and that `example::exit_code`
  answers exactly what the policy says.
- AC#3: `tests/features/outcomes.feature` with `tests/outcomes_bdd.rs` — three scenarios (a successful
  run, a question is never data, a failure writes a diagnostic and no data); the parity harness covers
  the cross-mode version, and the existing BDD suites already cover cancellation and refusals.
- TDD evidence: `tests/outcomes.rs` and `tests/outcomes_bdd.rs` were written first and failed to
  compile against a missing `clifacet::contract`; once it existed, two tests failed for real and both
  were fixed in the code rather than the test — the TUI genuinely drew on `stdout`, and it genuinely
  bypassed the passed diagnostic stream (the screen ended up on the test process's own stderr). Two
  further test-side mistakes were corrected: a scripted frontend cannot demonstrate where a question
  goes because it writes nothing (both the unit test and the BDD scenario now use a real `Terminal`
  with a shared buffer), and the BDD world gained the transcript it needs to assert that.
- Design decisions: the stream rule stays a rule (documented, enforced by the code and by tests) rather
  than becoming a `StreamPolicy` type, because the source of truth is which writer each call site is
  handed — a policy object would restate it without being able to enforce it. The exit mapping *is* a
  type, because it is a translation table with real behaviour and callers legitimately change it.
  `run_tui_on` is private: the public entry points keep their signatures, and `run_tui_with` remains
  the seam a test uses to assert a screen buffer separately from a diagnostic buffer.
- Coverage: `cargo llvm-cov` reports 1189/1194 source lines; the five uncovered lines are still the
  process's own terminal in `tui::app::run_on_with`.
- Deliberately out of scope: `--json` (the feature still only reserves its dependency) and the
  quiet/verbose flag family — no flag in the reference application changes the stream policy.
