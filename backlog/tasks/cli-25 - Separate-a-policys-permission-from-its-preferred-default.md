---
id: CLI-25
title: Separate a policy's permission from its preferred default
status: Done
assignee: []
created_date: '2026-11-01 17:22'
updated_date: '2026-11-01 17:26'
labels: []
milestone: m-5
dependencies: []
type: task
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A Policy only said how much interaction an application permits, so permission and default were the same value: the example permitted a TUI and therefore opened a form for a bare invocation. Add the missing half — the mode a run uses when the user asks for nothing — without changing what is permitted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Policy::prefer(Mode) states the default, and resolve uses it when the request is empty, stepping down only for capability.
- [x] #2 A preference above the ceiling cannot be declared: it is checked in a const fn, so a const policy fails to compile.
- [x] #3 With no preference stated, resolution is exactly what it was: the ceiling is the default.
- [x] #4 An explicit request still wins, and is still bounded by the ceiling rather than the preference.
- [x] #5 Policy::preference and Policy::default_mode make the pair readable, and docs/modes.md documents both with a compiled snippet.
- [x] #6 The example states Policy::up_to(Mode::Tui).prefer(Mode::Interactive), proven by a rstest-bdd scenario and by test-local.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TDD, in order:

1. Wrote the failing tests first in `crates/clifacet/tests/interaction_policy.rs`: six new table cases on `resolution_follows_request_then_policy_then_capability` (preference beats the ceiling; preference still steps down for capability; a preference less interactive than the ceiling; a headless environment overrides any preference; an explicit request beats the preference; a request below the preference is honored), plus `the_default_mode_is_the_preference_or_the_ceiling` (3 cases), `a_preference_above_the_ceiling_is_a_bug_not_a_default` and `a_preference_is_reported_as_written`.
2. Established why they failed: `error[E0599]` — no method named `prefer`, `preference` or `default_mode` on `clifacet::policy::Policy`; 12 errors, all the missing surface and nothing else.
3. Smallest change that passes: `Policy` gained `preference: Option<Mode>`, `prefer` (const, with the ceiling check), `preference` and `default_mode`; `resolve`'s no-request arm now steps down from `default_mode()` instead of the ceiling. 33 tests pass in that file.
4. Refactor without behavior change: clippy's `single_match_else` fired once the no-request arm became a block, so the arm became `if let ... else ...`; the two `impl Mode` blocks were merged so `asks_the_user` sits beside `rank`.

BDD (user-visible): a new scenario in `examples/simple/tests/features/interaction_policy.feature` — "An application that prefers prompts asks instead of drawing a screen" — with steps in `interaction_policy_bdd.rs` that give the run `Capabilities::terminal_with_tui()` and a scripted answer, then assert the answer becomes the data and no escape sequence reaches the diagnostic stream. It drives `example::main_with`, the public entry point, and passes.

Design decisions:

- **Permission and intent are separate fields, not a reinterpreted ceiling.** `up_to` keeps its meaning exactly, which is why every policy written before this change resolves identically (pinned by the untouched cases and by `default_mode`).
- **A preference above the ceiling cannot exist.** `prefer` asserts in a `const fn`, so a `const` policy fails to compile. Verified by hand with a throwaway test file: `error[E0080]: evaluation panicked: a policy cannot prefer a mode above its ceiling`. Not committed as a `trybuild` case, because that message is const-eval output that moves between rustc versions; the runtime `#[should_panic]` test pins the same sentence and the docs state the compile-time consequence.
- **`Mode::rank` is the single ordering.** `Ord`/`PartialOrd` are now implemented from it instead of derived, so the const comparison in `prefer` and the `<=` comparisons in `resolve` cannot disagree.
- **The example states the intent it wants**: `Policy::up_to(Mode::Tui).prefer(Mode::Interactive)`. Permissions are unchanged; a bare invocation now asks for what the flags left out instead of opening the form. Pinned by `the_application_asks_by_default_and_draws_only_when_asked` in `example_command.rs`.
- **Not done: choosing the mode from argv completeness.** "Non-interactive when all arguments are supplied" is already what the value level does — `Session::value(Some(v), ...)` never asks — and making the *mode* depend on `argv` would make `resolve` impure, would be vacuous for commands whose arguments all have defaults, and could not decide confirmations. `docs/modes.md` records that reasoning instead of leaving it implicit.
- **Not done: `default_mode` on `#[clifacet::command]`.** The attribute wraps a parser; the mode is application policy, decided against inputs the attribute never sees (the ceiling, the environment, the request). Declaring it twice would be a second source of truth for the same default, so `POLICY` stays the one place.

Docs: `docs/modes.md` gained "Permission and intent" (with a compiled snippet) and "Why \"all arguments are supplied\" is not a mode trigger"; `docs/command-core.md` now describes the example's policy rather than only its ceiling; the module doc in `policy.rs`, `docs/getting-started.md`, `examples/simple/README.md`, the root README and the changelog follow.

Gates: `test-local`, `skills-check`, `backlog-check`, `check-features` and `actionlint` are green.
<!-- SECTION:NOTES:END -->
