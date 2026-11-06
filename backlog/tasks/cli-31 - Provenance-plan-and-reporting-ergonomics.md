---
id: CLI-31
title: 'Provenance, plan and reporting ergonomics'
status: In Progress
assignee: []
created_date: '2026-11-01 22:34'
updated_date: '2026-11-06 15:13'
labels: []
milestone: m-6
dependencies: []
type: task
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Provenance derived from the parse result, a plan value every planned command can describe or execute, and one failure renderer. Decision-18 items 4, 5 and 7; item 6 (parity sugar) was deferred by decision-18's acceptance note and its AC removed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `Provenance::from_matches` records `Source::Argv`, `Source::Default` and `Source::Environment` for every present value, and records nothing for an absent one.
- [x] #2 A prompted value and a parsed value compose into one `Provenance`, and a secret is never recorded with a value.
- [x] #3 `Command::plan` returns a `Plan` that can be described or executed, and `execute` is the only execution path so the dry run cannot drift.
- [x] #4 A test asserts that a described plan and an executed plan agree, using the parity harness rather than a bespoke comparison.
- [x] #5 `report::render` writes a `Failure` with its kind and message, and adds the usage text for a usage failure, on the stream `ExitPolicy` implies.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Scope

Decision-18 items 4 (`Provenance::from_matches`), 7 (`report::render`) and 5 (`Command::plan`), built in that order (item 5 last, per the acceptance note). Item 6, the parity shorthand, was deferred by decision-18's acceptance note, so its AC (#6) was removed from this card on 2026-11-06 with the maintainer's agreement. The three design questions were answered by the maintainer on 2026-11-06, choosing the recommended option each time; recorded below.

## Decisions

- **`Provenance::from_matches(&clap::Command, &ArgMatches)`, not `from_matches(&ArgMatches)`.** Measured: `ArgMatches::ids()` lists the derive's argument group (named after the struct, here `Deploy`) beside the arguments, and the matches cannot tell the two apart. The first implementation recorded `Deploy <- argv`; walking the command's own arguments records values and nothing else. `try_contains_id` guards `value_source`, which panics on an id the matches were not built with. The chosen subcommand is walked; a `global` argument keeps one entry. `ValueSource` is non-exhaustive, and an unknown source is not guessed at.
- **Composition is explicit.** `with_secrets(names)` marks records secret without inventing a source for an absent value; `merge(&later)` adds a session's records, a later record replacing an earlier one for the same name. The session is not given the matches: provenance stays a value the application composes.
- **`report::render(program, report, &policy, out, err) -> i32`.** `Report` is `Failure(&Failure) | Clap(&clap::Error)` with `From` for both, so a call site passes `&failure` or `&error`. As the maintainer chose: a failure is written as `program: message`; its kind is carried by the exit code and not repeated in the text, so the documented output in `docs/command-core.md` is unchanged. A clap usage error is written as clap renders it (usage line included) on `err` with `policy.usage()`; help and `--version` go to `out` with `policy.success()`. Write errors are ignored, as the examples did.
- **`command::Planned` + `command::Plan<'a, T>`, not a provided `Command::plan`.** As the maintainer chose: `Plan` is steps (`Vec<String>`) plus a boxed `FnOnce` work closure; `step`, `steps`, `describe` (joined by newlines) and a consuming `execute`; `Debug` shows the steps only. A blanket `impl<P: Planned> Command for P` makes `execute` = `plan(input)?.execute()`, so for a planned command the single execution path is enforced by the type system, and existing `Command` impls are untouched (including `harness::Recording`). `Plan` lives in `clifacet::command`, not at the crate root, because `policy::Plan` already exists; the docs say so.

## Narrowings

- The plan's **steps are the command's words**, written beside the work from the same input. The library enforces one execution path, not that the words are true; `tests/plan.rs` shows the parity harness checking that (described steps against executed effects) and catching a plan whose description drifted.
- No session helper for dry runs: an application calls `Planned::plan` itself after collecting (backlog-lite does).
- Provenance does not record clifacet's own `fill` defaults (applied in the bridge, after the parse): such a value has no clap source and records nothing unless a session asked.
- **backlog-lite did not lose code overall**, against decision-18's "it should lose code". Non-comment lines are 501 → 503: the hand-written clap branch and both failure lines went (report), and a 15-line `Backlog` `Planned` impl arrived. `examples/simple` went 352 → 338.
- `Invocation`/`invocation_of` replace backlog-lite's own `Plan`/`plan_of` (unpublished example crate) to keep one `Plan` name per reader.

## Evidence

- AC#1/#2 red: `tests/provenance.rs` failed to compile (E0599: no `from_matches`, `with_secrets`, `merge`). Once implemented, `only_arguments_are_recorded` failed with `["Deploy", "dry_run", …]`, which led to the signature change above; 13/13 now pass, covering default, environment (`env = "PATH"`, no process-env mutation), absence, typed-beats-default/env, flags, subcommands, composition with a prompted value and a secret canary.
- AC#5 red: `tests/report.rs` failed on the unresolved `clifacet::report`; 7/7 pass (three failure kinds, a usage error with `Usage:`, help and version on the data stream). Both examples now report through it, and their full suites pass unchanged.
- AC#3/#4 red: `tests/plan.rs` failed on the unresolved `command::{Plan, Planned}`; 7/7 pass. The Debug expectation was corrected to `finish_non_exhaustive`'s `..`.
- Scenario: `examples/backlog-lite/tests/features/plan.feature` ("A dry run prints the plan, and only the plan") compares `bl --dry-run` output with `Backlog.plan(...)`'s description. It was written after the seam existed and passed when first run; it was not red first.
<!-- SECTION:NOTES:END -->
