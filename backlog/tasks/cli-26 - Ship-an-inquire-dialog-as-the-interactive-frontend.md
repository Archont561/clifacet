---
id: CLI-26
title: Ship an inquire dialog as the interactive frontend
status: Done
assignee: []
created_date: '2026-11-01 18:33'
updated_date: '2026-11-01 18:36'
labels: []
milestone: m-5
dependencies: []
type: task
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The interactive feature declared inquire as a dependency and nothing used it: the only prompt frontend was a plain question-and-line-reader. Ship the dialog the feature was reserved for, on the same two-method Prompter seam, so an application can choose its presentation without changing policy, streams or exit codes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 clifacet::prompt::Dialog implements Prompter behind the interactive feature: ask opens a Text dialog, confirm opens a Confirm dialog that defaults to no.
- [x] #2 Escape and Ctrl-C are FailureKind::Cancelled; a dialog that cannot reach a terminal is an execution failure naming the terminal rather than a question nobody can answer.
- [x] #3 The dialog decides nothing: a plan that may ask nothing never constructs it, a supplied value is never re-asked, and --yes answers confirmations without one appearing.
- [x] #4 The example uses it: run() asks through dialog_frontend(), terminal_frontend() stays beside it, and the crate enables the interactive feature.
- [x] #5 Docs, README, the example README and the changelog describe the three adapters and how an application ships its own; check-docs stays green with the new compiled snippet.
- [x] #6 test-local, check-features and alint are green, including the interactive-only and all-features configurations.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TDD, in order:

1. Wrote the failing tests first: `crates/clifacet/tests/dialog.rs` with four cases — a question without a terminal fails instead of waiting; a confirmation fails the same way; the dialog is what a `Session` asks through; a plan that asks nothing never builds it. Each is guarded by `if !stdin().is_terminal() { return; }` so a machine that *does* have a terminal skips rather than hanging the suite.
2. Established why they failed: `error[E0432]: unresolved import 'clifacet::prompt::Dialog'` — one error, the missing adapter, nothing else.
3. Smallest change that passes: `crates/clifacet/src/prompt/dialog.rs` with `Dialog`, `impl Prompter`, and a `failure(InquireError) -> Failure` mapping; `#[cfg(feature = "interactive")] mod dialog; pub use dialog::Dialog;` in `prompt.rs`.
4. Refactor without behavior change: the mapping is a free function so it can be unit-tested against every `InquireError` variant that can be constructed, instead of only the one a terminal-less process produces.

A red the gate found rather than the tests: the `Dialog` doctest called `.ask(..)` without `Prompter` in scope, so `test-doc` failed with `E0599` while `cargo test --test dialog` was green. Fixed by importing the trait in the doctest — a reminder that a doctest is compiled by a different target than the integration tests.

BDD, stated rather than assumed: this adapter adds no user-visible workflow of its own. The promises it participates in are already described by the example's `interaction_policy.feature` and `prompting.feature` — a question is asked only when the mode permits, a supplied value is never re-asked, `--yes` answers confirmations — and those scenarios drive `Scripted` through the same `Prompter` seam this adapter implements. A new scenario could not describe anything real: reaching the real dialog needs the process's own terminal, which the BDD harness deliberately cannot have. What is new is the error mapping, and that is pinned by unit tests over `InquireError` instead.

Design decisions:

- **`Dialog` decides nothing.** It is a `Prompter` and only a `Prompter`: whether a question may be asked stays in `Session`, which builds the frontend through `Plan::with_frontend`. That is why `a_plan_that_asks_nothing_never_builds_the_dialog` can assert the builder was never called, and why a richer widget cannot turn a piped run into an interactive one.
- **Confirmation defaults to no.** `Confirm::with_default(false)` matches `Session::confirm`'s "nothing able to ask returns `false`": a confirmation nobody answered must not read as consent.
- **Cancellation keeps its category.** Escape and Ctrl-C map to `FailureKind::Cancelled` (exit 130 in the example), not to an execution failure, so a wrapper can tell a decision from a breakage.
- **A missing terminal is an execution failure that says so.** It is the frontend that cannot run, and the message names the terminal; it is *not* the caller's input, so it is not `Validation`. This is also the only way the adapter can fail in CI, where `Capabilities::detected()` already resolves to `NonInteractive` before a dialog would be built.
- **`inquire` renders on `stderr`.** Verified in the vendored source (`IO::Std(stderr())`), so the stream contract is unchanged: nothing a person sees reaches the data stream.
- **Two frontends stay in the example.** `run()` asks through `dialog_frontend()`; `terminal_frontend()` is kept beside it and documented as the dependency-free reader. Two implementations of one seam is the point — the frontend is the application's choice, not a setting.
- **The feature gate is honest.** `interactive` was declared and unused; now it is the only thing that pulls `inquire` in, and `check-features` proves the crate still builds with no features, with `interactive` alone, and with everything.

Docs: `docs/command-core.md` gained the three-adapter table, the `Dialog` paragraph and a compiled snippet showing that a frontend choice cannot create consent; `README.md` names the dialog in the feature list and the module table; `examples/simple/README.md` says the interactive mode is a dialog and that the frontend is chosen in `src/lib.rs`; `docs/getting-started.md` and the changelog follow. `check-docs` regenerated `documented_snippets.rs` (12 snippets) unchanged in shape.

Evidence: `test-local` green — 35 unit + 252 integration Rust tests, 13 doc tests, 297 Python tests (2 skipped), 46 Alint rules, `check-docs`, `check-features` (no features, `interactive` alone, `tui` alone, `json` alone, all), `check-packages` and `sandbox-check`.
<!-- SECTION:NOTES:END -->
