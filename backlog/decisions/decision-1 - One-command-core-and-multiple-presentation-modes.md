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

## Amendment (2026-11-08, decision-19)

Registration stays explicit, and decision-19 item 4 defines one more explicit form: a
directory the application opts into from its own `build.rs` with `clifacet::build`. The
application wrote that call and chose that folder, and every command still compiles as a
module of its own source tree. Hidden global or link-time registration (`inventory`,
`linkme`, `ctor`) remains refused, as decision-15 item 3 measured.
