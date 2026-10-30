---
id: CLI-10
title: Add prompt adapter and scripted behavior tests
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-2
dependencies:
  - CLI-9
type: task
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provide an optional interactive adapter that shares the command core.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Missing values are prompted only when policy permits; supplied values are not needlessly re-asked.
- [x] #2 A scripted framework-owned seam tests successful input, cancellation and validation.
- [x] #3 Representative BDD scenarios demonstrate flag and prompt parity.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `clifacet::prompt` — a `Prompter` trait, a `Session` that decides
  whether a question may be asked, `Terminal` for a person and `Scripted` for tests — plus
  `example::run_with` / `example::main_with` so a flow can be driven with stated capabilities and
  a scripted frontend.
- AC#1: `Session::value` is the single rule — a supplied value is returned untouched, a missing one
  is asked only when the plan permits, and a non-interactive plan has no frontend at all because
  `Session::start` builds it through `Plan::with_frontend`. Tabled for the four combinations, with
  the script's `asked()` list asserted to be empty where nothing should have been asked.
- AC#2: the scripted seam is framework-owned (`prompt::Scripted`) and covers success, cancellation,
  a mismatched answer kind, an exhausted script and validation (through `Session::run` with a
  counting `Command`, which asserts zero executions when collection fails).
- AC#3: `tests/features/prompting.feature` with `tests/prompting_bdd.rs` — one scenario compares a
  prompted run with the equivalent flag run by asserting the same data *and* exit code; the other
  asserts a declined confirmation writes no data and exits 130.
- TDD evidence: `tests/prompting.rs` and the feature file were written before the module existed and
  failed to compile against `clifacet::prompt`. Two test-side mistakes surfaced and were fixed by
  correcting the test, not the code: "no" is an answer (`Ok(false)`) while a dismissal is the
  cancellation, and the cancellation scenario had to supply the name as a flag so the only question
  was the confirmation.
- Design decisions: `Session::run` collects and then executes, which makes "nothing runs unless
  collection succeeded" structural; `policy::invoke` retired into it, so there is one path from
  request to run rather than two. `Terminal` asks on the *diagnostic* stream, treats end of input as
  a cancellation rather than a blank answer, and re-asks on an unclear confirmation. `--yes` only
  short-circuits `Session::confirm`; a confirmation with nothing able to ask returns `false`, so a
  caller that forgets the plan fails safe.
- Deliberately out of scope: re-prompting a value that fails validation (one question, one answer —
  re-asking belongs with a real frontend's UX work), the TUI (CLI-11), and secret redaction of
  prompt answers (CLI-14).
