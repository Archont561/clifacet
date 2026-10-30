---
id: CLI-14
title: Implement secret handling and provenance
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-4
dependencies:
  - CLI-7
  - CLI-13
type: task
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define and test framework-owned secret and provenance guarantees.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Framework diagnostics, previews and test records redact supplied secret values.
- [x] #2 Input provenance records source without the secret value; failed tests do not print canaries.
- [x] #3 Limitations for application-owned logs and unsafe argv sources are documented.
<!-- AC:END -->

## Notes

- 2026-10-31: delivered as decision-12 — redaction by type rather than by scrubbing. `secret::Secret<T>`
  redacts its own `Debug` and `Display`, so every framework-owned surface (a diagnostic, a harness
  record, a form preview, a failing test's panic message) is covered without keeping a list of live
  secret values anywhere. `provenance::{Source, Origin, Provenance}` records where a value came from;
  an `Origin` has no field for a value, so the record cannot leak one. The example gained `--token` and
  refuses a credential beginning with `revoked`; `FieldState::mark_secret` / `FormState::mark_secret`
  make a form preview draw the marker.
- AC#1: three surfaces, each with a test. Diagnostics — `example` interpolates the `Secret` itself into
  its rejection message, and `tests/secret_redaction.rs` asserts the canary is absent from stderr while
  `[redacted]` is present, end to end through `main_with`. Previews — a secret field draws
  `[redacted]` in a `TestBackend`, reports no length, and still submits the value. Test records — a
  forced `Parity` disagreement whose effects are `format!("{input:?}")` panics without the canary in
  the message, which is checked by catching the panic.
- AC#2: `Provenance` holds a name, a `Source` and a `bool`; there is no value field, so "records the
  source without the secret value" is structural rather than a rule. Re-recording a name replaces the
  entry instead of appending, so a re-ask does not inflate the record. Canary discipline: every
  leakage assertion names only what is missing, never the text it inspected, so a failing redaction
  test cannot print the value it protects. One earlier draft of the form test did print it via
  `assert_eq!` and was corrected.
- AC#3: `docs/command-core.md` gains "Secrets and provenance" with a "What this cannot do" list —
  application-owned logs behind `expose()`, argv being visible in `ps`/`/proc`/shell history,
  environment variables being readable by the same user, a prompted answer being echoed by the
  terminal, plain `String` credentials, and third-party diagnostics. The stale "no input is marked
  secret" limitation is replaced.
- TDD evidence: `tests/secret_redaction.rs`, `tests/secret_bdd.rs` and `tests/features/secret.feature`
  were written first and failed to compile against missing `clifacet::secret`, `clifacet::provenance`
  and `ExampleInput::token`; three test-side mistakes were then fixed by correcting the test — the
  submission assertion compared the whole argument vector (which also carries `--name`), the form
  masking test printed the canary in its failure message, and the blank-credential case needed
  `clap::Parser` in scope rather than an inherent `try_parse_from`.
- Signature change to flag: `ExampleInput` gained `token: Option<Secret<String>>`, so every struct
  literal for it was updated (unit tests, `example_command`, `facet_bridge`, `properties`,
  `harness_parity`) and the parity test's exact recorded representation now ends `level: Basic, token:
  None`. The field is declared last so an existing record reads in its original order.
- Coverage: `cargo llvm-cov` reports 1145/1150 source lines. The five uncovered lines are the
  process's own terminal in `tui::app::run_on_with`, unchanged since CLI-11.
- Deliberately out of scope: no-echo prompting, secret-aware environment handling, and scrubbing of
  strings the application built itself — all three are documented boundaries, not omissions.
