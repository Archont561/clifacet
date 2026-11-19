---
id: CLI-73
title: Machine-readable CLI schema introspection
status: To Do
assignee: []
created_date: '2026-11-19 14:30'
labels: []
milestone: m-19
dependencies:
  - CLI-72
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement machine-readable CLI schema introspection behind the `schema` Cargo feature (decision-25). Introduce `mycli schema` / `--schema` command that reflects the application's command hierarchy, subcommands, arguments, options, types, required/optional flags, default values, environment variable bindings, and docstrings into an introspectable JSON Schema specification.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 schema Cargo feature and CliMetadata reflection types are defined
- [ ] #2 mycli schema emits complete command hierarchy, argument constraints, and docstrings as JSON Schema
- [ ] #3 Macro #[clifacet::app] conditionally generates the schema command when feature is active
- [ ] #4 Integration tests verify schema accuracy against declared commands and argument types
<!-- AC:END -->
