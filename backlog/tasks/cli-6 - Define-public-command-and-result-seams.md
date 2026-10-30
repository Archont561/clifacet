---
id: CLI-6
title: Define public command and result seams
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-1
dependencies:
  - CLI-1
type: task
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Establish a minimal UI-free command execution and output contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A domain-neutral Example command executes without a terminal or UI dependency.
- [x] #2 Application-owned output and exit mapping distinguishes success and validation or execution failure.
- [x] #3 Public seams and known unsupported behavior are documented and tested.
<!-- AC:END -->

## Notes

- 2026-10-30: delivered as `Command` + `Failure`/`FailureKind` + the `example` reference application, with
  `docs/command-core.md` for the seams and the unsupported list. Verified on PR #11: 25 crate tests (16
  integration through buffers, 7 unit, 2 doctests), clippy `-D warnings` over all targets and features,
  `check-features` green across base/interactive/tui/json, and CI's MSRV job compiling all of it at 1.88.
- AC#1 is proven by construction rather than by a TTY-less process test: `example::main` takes its argv and
  both streams as parameters, so no test can reach a terminal even by accident.
- Two bounds are worth carrying forward: `--repeat 255` parses as a `u8`, so range validation is the
  library's job and is tested as such; and help/`--version` exit 0 on the *data* stream, because
  `clap`'s `use_stderr` says they are the requested result rather than a diagnostic.
- `clap` is re-exported from the crate: `example::Cli` implements its traits, so a caller resolving a
  different `clap` would be a version mismatch rather than a compile error they can read.
- Deliberately left to their own tasks: the companion derive and `trybuild` cases (CLI-7), rstest/proptest
  foundations (CLI-8), mode resolution and cancellation (CLI-9), secrets (CLI-14).
