---
id: decision-3
title: Interaction policy and common result contract
date: '2026-10-29 15:28'
status: accepted
---
## Context
The same command must behave predictably with flags, prompts and a TUI, including CI and machine consumers.

## Decision
Resolve runtime mode separately from Cargo features. Precedence: explicit request, application policy, then detected capabilities. A policy states a **ceiling** (what the application permits at all) and may state a **preference** (what it uses when the user asked for nothing); the two are not the same value, and a preference can never exceed the ceiling. Forbidden modes must not construct a prompter; `--yes` applies to confirmations only. Keep output and exit mapping application-owned, with stdout reserved for requested machine data. Use a single final typed result and shared validation/execution.

## Consequences
Lock down precedence, JSON behavior, cancellation and errors with public-seam tests before declaring stability. Avoid treating TTY detection as user consent to launch a TUI. Plan/confirm/dry-run is post-0.1 unless explicitly brought forward.

## Acceptance note

2026-11-01: accepted, with `0.1.0` shipped against it. Each clause of the Decision has a
place where it is decided once and tested at a public seam:

- **Precedence** is `Policy::resolve(Request, Capabilities) -> Result<Plan, Refusal>` — one
  pure function over three inputs, with `Request` and `Capabilities` as separate values so
  "the user asked" can never be confused with "the environment allows". CLI-9 built it,
  CLI-25 split permission from preference inside it, and
  `crates/clifacet/tests/interaction_policy.rs` plus the parity harness (CLI-13) hold the
  matrix.
- **A forbidden mode never constructs a prompter.** Resolution happens before any frontend
  exists and reports a typed `Refusal` (`Forbidden` above the ceiling, `Unavailable` when
  the environment cannot host the request), and `Plan::with_frontend` is the only door to
  building one — it returns `None` for a plan that asks nothing, so a non-interactive run
  cannot build a prompt by accident.
- **`--yes` applies to confirmations only.** It is carried on `Request::assuming_yes` and
  answers questions of the confirm kind; it never selects a mode and never fills a field.
- **Output and exit mapping stay application-owned.** `contract` names the streams and maps
  each outcome to a code; the example owns the constants (`clifacet_example::EXIT`) and the
  library writes nothing itself. CLI-15 finished this, including the deliberate overlap
  between usage and validation exit codes.
- **One final typed result.** Every frontend returns the same `Result<Output, Failure>`;
  cancellation is a `FailureKind::Cancelled` produced by the prompt and dialog adapters
  alike (CLI-26), not a special exit path.

Two clauses are *not* closed by this acceptance, and both were always outside `0.1.0`:

- **JSON behavior.** The `json` feature reserves `serde_json`, and no flag selects it yet,
  so there is no serialised contract to lock down. Adding the flag is what would make this
  clause testable; until then the crate documents the gap instead of implying support.
- **Plan/confirm/dry-run.** Still post-0.1, as the Consequences said.
