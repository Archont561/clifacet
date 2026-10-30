---
id: CLI-9
title: Implement deterministic interaction policy
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-2
dependencies:
  - CLI-7
type: task
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Resolve runtime mode based on explicit request, app policy and capabilities.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Policy is deterministic and covered by rstest decision tables.
- [x] #2 Forbidden modes do not construct a prompter; yes affects confirmations only.
- [x] #3 Cancellation and invalid explicit requests are distinct and do not execute the handler.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `clifacet::policy` — `Mode`, `Capabilities`, `Request`, `Policy`,
  `Plan`, `Refusal` and `policy::invoke` — plus `FailureKind::Cancelled` and the example's flags
  (`--interactive`, `--tui`, `--yes`, grouped in `InteractionFlags`). Runtime mode is resolved
  separately from Cargo features, as decision-3 asks.
- AC#1: `tests/interaction_policy.rs` carries a ten-row decision table over
  (request, policy ceiling, capabilities), four rows for `--yes`, three for which modes ask the
  user, and a refusal-message table. `Policy` is a *ceiling*, not a preference list, so the state
  space has no invalid combinations: interaction can only ever decrease.
- AC#2: `Plan` is the only value that can build a frontend, and `Plan::with_frontend` answers
  `None` when the mode asks nothing. A refused request yields no `Plan` at all, so "forbidden modes
  do not construct a prompter" is asserted with a construction counter that must stay at zero — a
  structural claim, not a convention. `--yes` is tabled against four mode/flag combinations and
  changes only `assumes_yes`, never the mode.
- AC#3: cancellation is `FailureKind::Cancelled` with `Failure::cancelled`, mapped by the example to
  its own exit code `130`; `a_refused_request_never_executes_the_handler` runs a counting `Command`
  through `policy::invoke` and asserts zero invocations for refused and unavailable modes. A
  cancelled frontend never reaches `invoke` at all, because it cancels while collecting the input —
  the structural reason the category can be distinct without a frontend existing yet.
- TDD evidence: the tables and the BDD steps were written first and failed to compile against a
  `clifacet::policy` that did not exist and a `Failure::cancelled` that did not exist. Then the
  implementation, then clippy's own findings (four bools on one struct, identical match arms) were
  fixed by changing the design — the flags became `InteractionFlags`, `missing_for` became one
  predicate — rather than by allowing the lints.
- BDD evidence: `tests/features/interaction_policy.feature` with `tests/interaction_policy_bdd.rs`,
  two scenarios driven through `example::main`: a refused `--tui` writes nothing to the data stream
  and exits 2 with a diagnostic naming the TUI; `--yes` leaves the mode alone and still prints data.
- Design decision recorded: the example's ceiling is `Mode::NonInteractive`, so `--tui` and
  `--interactive` are refused with a message instead of resolving to a mode no frontend can host.
  CLI-10 raises the ceiling by one line; the capability plumbing it needs is already tested.
- Deliberately out of scope: TTY/TERM detection beyond "is stdout a terminal" (a false positive would
  resolve to a mode that cannot render), and the prompt and TUI frontends themselves (CLI-10, CLI-11).
- 2026-10-30 (CLI-10 follow-up): `policy::invoke` was folded into `prompt::Session::run`, which now
  owns "collect, then execute" for both flag and prompted runs. The refusal half of AC#3 is unchanged
  — a refused request still produces no `Plan`, so no session and no execution — and the
  "handler not executed" assertion moved to `tests/prompting.rs` against the new seam. One path from
  request to run, rather than two.
