---
id: decision-3
title: Interaction policy and common result contract
date: '2026-10-29 15:28'
status: proposed
---
## Context
The same command must behave predictably with flags, prompts and a TUI, including CI and machine consumers.

## Decision
Resolve runtime mode separately from Cargo features. Proposed precedence: explicit request, application policy, then detected capabilities. Forbidden modes must not construct a prompter; `--yes` applies to confirmations only. Keep output and exit mapping application-owned, with stdout reserved for requested machine data. Use a single final typed result and shared validation/execution.

## Consequences
Lock down precedence, JSON behavior, cancellation and errors with public-seam tests before declaring stability. Avoid treating TTY detection as user consent to launch a TUI. Plan/confirm/dry-run is post-0.1 unless explicitly brought forward.
