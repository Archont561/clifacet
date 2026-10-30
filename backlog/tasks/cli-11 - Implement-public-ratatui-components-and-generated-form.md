---
id: CLI-11
title: Implement public ratatui components and generated form
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-3
dependencies:
  - CLI-10
type: task
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provide a minimal TUI from reusable public ratatui components.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generated form uses the same public components available to consumers.
- [x] #2 Headless tests cover input, focus, submission and cancellation.
- [x] #3 TUI submission shares the common validation and handler path.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `clifacet::tui` — `state` (`FormState`, `FieldState`, `Key`, `Outcome`),
  `widgets` (`Form`, `Field`) and `app` (`run`, `run_on`, `live_keys`, `translate`) — behind the
  existing `tui` feature, with `ratatui` re-exported as `tui::ratatui` so a consumer (and a test
  backend) needs no direct dependency.
- AC#1: `tui::Form` renders nothing but `tui::Field`s, and both are public; `example::tui_form`
  builds the example's screen with `FormState::generated`, so the generated form and a hand-written
  one share the same code. `tests/tui_form.rs` renders the whole form through `Form` and a single
  `Field` on its own, which is the builder path an application uses for a custom layout.
- AC#2: `tests/tui_form.rs` (16 cases) drives a `TestBackend` plus a `Key` list for typing,
  backspace, focus wrapping, submit, cancel, argument building, the refusal message and the drawing
  itself; `tests/tui_keys.rs` maps terminal events without a terminal. No test needs a TTY.
- AC#3: `example::tui_submit` turns the answers into arguments, parses them with
  `Cli::try_parse_from` and then calls `Facet::validate` and `Example::execute` — the same path
  `--name ada` takes. `tests/tui_parity.rs` asserts the typed `ada` writes byte-for-byte the flag's
  data, and that a refused `--repeat 9` / `--repeat x` keeps the form open with exactly the words
  the flags print.
- BDD: `tests/features/tui.feature` with `tests/tui_bdd.rs` — one scenario compares a typed answer
  with the flag run (data and exit code), the other asserts escape writes nothing and exits 130.
- Decision references: decision-4 (layered components, custom escape hatches, no universal
  widget-backend abstraction, headless tests, UI adapters gated by runtime features) and
  decision-3 (the mode is resolved against capabilities, so `--tui` is honoured only when the
  feature is compiled *and* there is a terminal).
- TDD evidence: `tests/tui_form.rs`, `tests/tui_parity.rs`, `tests/tui_bdd.rs` and the feature file
  were written first and failed to compile against a nonexistent `clifacet::tui`; the modules then
  grew until they passed. Three test-side mistakes were fixed by correcting the test: the refused
  case compared a `--repeat x` answer against a `--repeat 9` flag run, `ratatui::Widget` lives in
  `ratatui::widgets`, and one isolation assertion had its expected value on the wrong field.
- Design decisions: the state machine is `ratatui`-free (`Key` is not a crossterm event) so focus
  and editing are assertable headlessly; which arguments a screen asks for is named explicitly
  rather than guessed from the grammar, because the mode flags are not questions; a prefilled value
  is replaced by typing and edited by backspace, since appending to a default turns "change 1 to 3"
  into "delete the 1 first"; an empty optional value is omitted from `arguments()` so an untouched
  field behaves as if the flag was never typed; `app::run_on` leaves the alternate screen on every
  path and treats raw mode as best-effort, because a buffer has no line discipline to switch off.
- Coverage: `cargo llvm-cov` reports 815/820 source lines. The five uncovered lines are the
  process's own terminal — the size probe, the terminal-open error and the raw-mode restore — which
  no headless test can reach; everything they wrap is covered, and `--tui` dispatch is exercised
  through `main_with` with a `tui`-capable `Capabilities`.
- Deliberately out of scope: bespoke fields beyond a labelled value, whole-screen escape hatches and
  theming (CLI-12), the cross-mode parity harness (CLI-13) and secret redaction (CLI-14).
