---
id: CLI-30
title: Choice and classified-confirmation questions
status: Done
assignee: []
created_date: '2026-11-01 22:34'
updated_date: '2026-11-06 14:01'
labels: []
milestone: m-6
dependencies: []
type: task
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Session and Prompter can ask a multi-choice question, a secret, and a repeated value, and can tell an answered confirmation from one nothing could ask. Decision-18 items 1-3; the backlog-lite wrapper is the acceptance case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `Prompter::choose` takes labelled choices and `Session::choose` returns a supplied value untouched, so prompting never overrides an explicit answer.
- [x] #2 The `inquire` frontend offers the choices as a selectable list, and the scripted frontend answers them, with a behaviour test for both.
- [x] #3 `Session::ask_secret` asks without echoing, records `Source::Prompt` with the secret flag set, and the value is reachable only through `Secret`.
- [x] #4 `Session::collect_values` repeats one question until the user stops, and a partially collected list is never returned as success.
- [x] #5 `Session::confirm` classifies the outcome as asked-and-declined, assumed by `--yes`, or unavailable, while the boolean fail-safe behaviour is unchanged.
- [x] #6 `examples/backlog-lite` stops inspecting `session.plan()` by hand and its status filter becomes a choice prompt; its tests stay green or get simpler.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-06 — implemented test-first on arena/89a3ccf7-clifacet (decision-18 accepted as suggested; items 1–3).

Decision record: decision-18 "Implementation note for items 1–3" lists every difference from the proposal (choose returns a position; Choice has label+help, no value; provided methods; ask_secret's default refuses; Confirmation has four variants and is breaking; the session keeps its own Provenance; string-only collect_values; bare --status only).

Failing-test-first evidence (each red run observed before the change):
- AC#5: E0432 unresolved `clifacet::prompt::Confirmation`. backlog-lite `yes_settles_the_confirmation_even_when_nothing_can_ask`, run against the UNCHANGED code, failed with "refusing to move CLI-20 to Done unconfirmed: pass --yes to say so in advance". That is a real bug: `bl done X --yes` without a terminal was refused, because the old code checked the mode before --yes.
- AC#1/#2: E0407 (`choose` not a member of `Prompter`), E0432 `Choice`, E0599 `Session::choose` ×8; dialog unit test E0432 `select`.
- AC#3: E0599 `Session::ask_secret` ×6, `Session::provenance` ×7, `Terminal::ask_secret`; dialog E0432 `password`.
- AC#4: E0599 `Session::collect_values` ×5.
- AC#6: 3 flows failed, because clap rejected a bare `--status` (InvalidValue).

Narrowings (stated, not silent):
- AC#2: inquire's test backend is pub(crate), so no test can press keys in a real Select or Password. The inquire half is asserted on what is reachable: the Select we build (labels in order, help beside them, cursor on the first), the hidden one-shot Password, the index mapping (inquire documents ListOption.index as relative to the original list), and the no-terminal failure for choose and ask_secret. This is the same boundary as Esc in docs/command-core.md.
- AC#3: "asks without echoing" holds for Dialog. Terminal and any frontend without an override refuse (Execution, "cannot ask … without echoing") before showing the question or reading a byte. The derive still refuses `ask` on a `secret` field.
- AC#6: the status filter is a choice only when `--status` is given without a value, because `bl list` is the default verb and must not grow a question. Headless, a bare `--status` is a validation failure naming every status.
- Not in CLI-30: the tui form's choice field (decision-18 consequence), typed collect_values, and passing help through Session::choose.

Tests: prompting.rs 69 (choices, terminal menu, default-method fallback, out-of-range frontend, secrets incl. leak checks, lists incl. partial failure); declared_fields +2 (ask_for records prompt / rejected answer records nothing); dialog lib +2, dialog integration +2; backlog-lite flows 19 (+6) and questions_bdd 5 scenarios (new feature file; rstest-bdd added as a dev-dep, lockfile entry only); example secret_bdd +2 and prompting_bdd +1 scenarios. test-local exit 0: 67 + 393 nextest, 319 pytest.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed on maintainer authorisation (2026-11-06). All six ACs met and merged in PR #49 (`9a44d440`): `Choice`, `Prompter::choose`/`ask_secret`, `Session::{choose, ask_secret, collect_values, confirm -> Confirmation, provenance}`, and backlog-lite's status filter as a choice prompt. Narrowings recorded in the notes: inquire prompts are not key-driven in tests, only a bare `--status` prompts, and `collect_values` is untyped.
<!-- SECTION:FINAL_SUMMARY:END -->
