---
id: CLI-43
title: Hide a secret's environment value from --help
status: Done
assignee: []
created_date: '2026-11-08 01:58'
updated_date: '2026-11-08 12:18'
labels: []
milestone: m-11
dependencies: []
type: task
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
clap's help prints an environment variable's current value beside its name (`[env: TOKEN=…]`), so a credential declared `secret` and read from the environment was shown by `--help`, against decision-12's promise that the framework never shows one. Found in the design review that led to decision-19 and confirmed end to end through `#[clifacet(env = "PATH", secret)]`, whose help printed `[env: PATH=/home/user/clifacet/…]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A field declared `#[clifacet(.., secret)]` gets clap's `hide_env_values = true`, so its help names the variable and never shows the value, proven by a test that reads a variable every platform sets rather than mutating the process environment
- [x] #2 `#[derive(Facet)]` refuses a `#[facet(secret)]` field whose `#[arg(env)]` would show the value — no `hide_env_values` or `hide_env` beside it — with a trybuild case whose message says what to write, and a hidden one still compiles
- [x] #3 The secrets table in docs/command-core.md lists `--help` as a covered surface and says how
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Red first: `help_names_a_secret_environment_variable_but_never_shows_its_value` (crates/clifacet/tests/declared_command.rs) failed before the fix, on its first assertion, because the help read `[env: PATH=/home/user/clifacet/…]` — the leak, end to end through `#[clifacet(long, env = "PATH", secret)]`. `PATH` stands in for the variable because every platform sets it, so no test mutates the process environment (the workspace forbids `unsafe`, and edition 2024 makes `set_var` unsafe).

Fix, in the smallest place each path allows:
- `#[clifacet(.., secret)]` (crates/clifacet-macros/src/field.rs) adds `hide_env_values = true` to the forwarded `#[arg(..)]`. It is a no-op on a field that reads no variable and also covers a raw `#[arg(env)]` on the same field.
- `#[derive(Facet)]` cannot change clap's attributes, so it refuses: a `#[facet(secret)]` field whose `#[arg(env)]` has neither `hide_env_values` nor `hide_env` (bare or `= true`) is a compile error saying what to write (crates/clifacet-macros/src/rules.rs). trybuild: `tests/ui/facet_secret_env_shown.rs` fails with that message; `tests/ui/facet_valid.rs` gains a hidden secret that still compiles.

Narrowing (AGENTS.md): this repairs an existing promise (decision-12 redaction) rather than adding a workflow, so it is proven by an integration test and trybuild cases, not a new scenario. Design: no new public item; the derive refuses rather than guesses, as its other field rules do. Compatibility: an application that declared `#[facet(secret)]` with a visible `#[arg(env)]` now fails to compile — on purpose, because its `--help` was printing the credential.
<!-- SECTION:NOTES:END -->
