---
id: decision-1
title: One command core and multiple presentation modes
date: '2026-10-29 15:27'
status: accepted
---
## Context
A CLI library must support non-interactive, prompt and TUI experiences without duplicating command behavior.

## Decision
Keep one UI-free command core and shared validation/execution path. Runtime modes are presentation adapters, not separate business logic. Registration is explicit; derive macros provide metadata and adapters rather than hidden global registration.

## Consequences
Applications can choose the presentation without rewriting handlers. Tests must exercise parity at public seams. No universal widget backend or arbitrary wizard graph is required.
