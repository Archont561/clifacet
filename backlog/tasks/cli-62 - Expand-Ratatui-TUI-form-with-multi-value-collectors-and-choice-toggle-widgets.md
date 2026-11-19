---
id: CLI-62
title: Expand Ratatui TUI form with multi-value collectors and choice/toggle widgets
status: Done
assignee: []
created_date: '2026-11-19 08:20'
updated_date: '2026-11-19 09:30'
labels: []
milestone: m-16
dependencies: []
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Expand Ratatui full-screen form components (crates/clifacet/src/tui) so that repeatable flags (ArgAction::Append), multiple positional values, choice selectors, and toggles match interactive prompt capabilities (decisions 4, 18, and 19). Multi-value fields can be input and submit multiple flag instances into clap parsing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Repeatable arguments (ArgAction::Append and num_args > 1) become multi-value form fields instead of being refused
- [x] #2 Form submission preserves multiple entries as separate CLI arguments (e.g. --tag a --tag b)
- [x] #3 Choice fields render options clearly and allow navigation without losing existing form input
- [x] #4 Toggles display boolean checkbox states and submit bare flags when flipped from defaults
- [x] #5 Unit, headless integration, and parity tests verify multi-value form collection and submission
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- Extended `Kind` in `crates/clifacet/src/tui/state.rs` with `Kind::List { choices: Vec<String> }` to collect multiple values for repeatable flags (`ArgAction::Append`) and multi-value arguments (`num_args > 1`), rather than refusing them with an execution error.
- Introduced `Placement` (`Flag(Option<String>)` vs `Positional`) in `FieldState` to support submitting positional values bare, while preserving single or multi-value flags under their respective long/short flags without triggering `clippy::struct_excessive_bools`.
- Added `FieldState::list()`, `FieldState::values()`, `FieldState::is_list()`, `FieldState::is_positional()`, and `FieldState::mark_positional()` helpers.
- Updated `crates/clifacet-macros/src/rules.rs` to allow `form` on list fields (`Vec<String>`), and removed obsolete trybuild rejection `field_form_list.rs`.
- Documented repeatable and list form fields in `docs/guides/derive.md` and `docs/reference/command-core.md`.
- Added unit and integration tests across `tui_form.rs` and `declared_command.rs` asserting multi-value input, positional submission, and clap parsing parity.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All five acceptance criteria satisfied:
1. `ArgAction::Append` and `num_args > 1` arguments now generate `Kind::List` multi-value form fields.
2. Form submission emits each entry as separate CLI arguments (e.g., `["--tags", "a", "--tags", "b"]`) or bare positional tokens.
3. Choice fields expose choices via `.choices()` and cycle cleanly without loss of state.
4. Toggles continue rendering checkboxes and submitting bare flags only when differing from defaults.
5. All tests (unit, integration, and parity) pass cleanly.
<!-- SECTION:FINAL_SUMMARY:END -->
