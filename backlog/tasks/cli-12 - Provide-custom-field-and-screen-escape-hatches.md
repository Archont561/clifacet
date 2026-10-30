---
id: CLI-12
title: Provide custom field and screen escape hatches
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-3
dependencies:
  - CLI-11
type: task
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Demonstrate composition without a separate command core.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A custom field widget can participate in a form.
- [x] #2 A custom screen can submit a draft through the same final validation.
- [x] #3 Examples and headless tests demonstrate both supported escape hatches.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as the two hatches decision-4 names, on the components CLI-11 shipped.
  `tui::CustomField` is a trait an application implements to draw one field; `Form::with_field` hands
  a form the overrides it should use. `app::run_with` replaces the whole drawing, and `run_on_with`
  does the same inside the alternate screen, so a custom screen is not left to reimplement entering
  and leaving it.
- AC#1: `Form::with_field(name, &custom)` draws that field with the application's widget and every
  other field with the plain `Field`. Nothing is registered: the overrides belong to one `Form`
  value, so the same application can draw the same field two ways on two screens. An override that
  names a field the form does not have is ignored, which keeps the field list the only source of
  truth for what exists.
- AC#2: `app::run_with` changes the drawing and nothing else — the same keys, the same refusals and
  the same `submit` callback — so a custom screen reaches `clap`, `Facet::validate` and the handler
  through `example::tui_submit`, exactly as the generated form does. `tests/tui_escape_hatches.rs`
  asserts the typed `ada` produces the same data through a hand-drawn screen, and that a rejected
  `--repeat 9` leaves the same message on it.
- AC#3: `examples/tui_custom_screen.rs` runs both hatches at once (a masked name field inside a
  screen of the application's own); `tests/tui_escape_hatches.rs` covers them headlessly and
  `tests/features/tui_escape_hatches.feature` with `tests/tui_escape_hatches_bdd.rs` states them as
  scenarios.
- Decision references: decision-4 (explicit hatches, no universal widget-backend abstraction, no
  global registration, headless tests) and decision-3 (whatever draws the screen, the plan and the
  submission boundary are unchanged).
- TDD evidence: `tests/tui_escape_hatches.rs` was written first and failed to compile because neither
  `CustomField` nor `run_with` existed; it then failed for real on three test-side mistakes that
  were fixed by correcting the test rather than the code — `Widget::render` takes `self` and is not
  callable on a trait object (so the hatch draws into the buffer instead of returning a boxed
  widget), the example's `--name` has no clap default (the `world` default lives in validation), and
  a crossterm backend writes a row as several cursor moves, so the drawn text is not contiguous in
  the output stream.
- Design decisions: the hatch draws into `(area, buffer)` rather than returning `Box<dyn Widget>`
  because ratatui 0.30 renders boxed widgets through the unstable `WidgetRef` feature, and pinning an
  unstable feature to pass one argument would be a poor trade; `Form` is a column of one-row
  rectangles, which is what gives a custom field an area of its own and keeps a short terminal
  drawable instead of panicking; the custom drawing is a `Fn` (not `FnMut`) so a screen cannot carry
  hidden state between frames without the application owning it.
- Coverage: `cargo llvm-cov` reports 860/865 source lines. The five uncovered lines are the
  process's own terminal in `app::run_on_with` — the size probe, the terminal-open error and the
  raw-mode restore — which no headless test can reach.
- Deliberately out of scope: theming and more widgets, a cross-mode parity harness (CLI-13) and
  secret provenance (CLI-14) — the masked field here is a drawing demonstration, not a promise that
  values are never echoed.
