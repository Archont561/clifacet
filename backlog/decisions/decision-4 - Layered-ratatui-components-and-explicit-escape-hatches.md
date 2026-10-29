---
id: decision-4
title: Layered ratatui components and explicit escape hatches
date: '2026-10-29 15:28'
status: accepted
---
## Context
Generated forms are useful only if applications can replace pieces without a separate command implementation.

## Decision
Ship ratatui-specific public components, and build generated forms from those components. Support custom field widgets and a custom whole-screen path that returns to the same validation and execution boundary. Do not introduce a universal widget-backend abstraction or global registration.

## Consequences
Test the generated form and overrides with a headless backend. Framework macros emit UI-neutral metadata; UI adapters are gated by runtime crate features.
