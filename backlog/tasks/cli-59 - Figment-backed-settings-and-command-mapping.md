---
id: CLI-59
title: 'Figment-backed configuration engine and command function mapping'
status: To Do
assignee: []
created_date: '2026-11-18 15:00'
labels: []
dependencies:
  - CLI-51
  - CLI-52
type: task
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements decision-22: integrating Figment as an optional, first-class configuration engine for clifacet, mapping each command function (`#[clifacet::command] pub fn cmd(...)`) to a configuration table (`[cmd]`, `[group.cmd]`).

Provides `FigmentLoader` and `App::figment(...)` behind an optional `settings-figment` feature, derives `serde::Deserialize` and `serde::Serialize` on generated command `*Input` structs under the `serde` feature, normalizes kebab-case and snake_case keys, and preserves clifacet's precedence (`flag > environment > config file > built-in default`), interactive/TUI pre-population, and provenance tracking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An optional Cargo feature `settings-figment` provides a Figment-backed settings loader that maps nested command paths (`serve`, `db.migrate`) to Figment dictionary tables
- [ ] #2 Precedence follows flag > environment > Figment profile/layers > built-in default; CLI flags override Figment values, and Clap's value parsers validate injected defaults
- [ ] #3 `#[clifacet::command]` on a function derives `serde::Deserialize` and `serde::Serialize` on the generated `*Input` struct when the `serde` feature is active, enabling direct `figment.extract_inner::<Input>(command)`
- [ ] #4 Key normalization transparently resolves kebab-case CLI flags and snake_case config keys (e.g. `listen-port` <-> `listen_port`)
- [ ] #5 Provenance correctly records `Source::File` with path attribution when values are supplied by Figment file providers
- [ ] #6 Interactive prompts and TUI forms receive Figment-configured defaults as initial values
- [ ] #7 The dependency graph and licenses for Figment and its transitives (`uncased`) pass `cargo deny` and satisfy repository audit policy
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision-22 outlines the hybrid architectural model: Figment layers multi-source configuration files, environment variables, and profiles, feeding scalar defaults into Clap before argument parsing, while also allowing direct Serde deserialization of command input structs.

**Proposed Components:**
- `crates/clifacet/src/settings/figment.rs`: `FigmentLoader` implementing `clifacet::settings::Loader` by extracting entries from a `figment::Figment` instance.
- `crates/clifacet/src/app.rs`: `App::figment(self, figment: figment::Figment) -> Self` runner integration.
- `crates/clifacet-macros/src/function.rs`: update generated `*Input` struct derives to include `serde::Deserialize` and `serde::Serialize` when `cfg(feature = "serde")`.
- `crates/clifacet/tests/settings_figment.rs`: integration tests demonstrating file merging, profile selection, CLI overrides, and command function mapping.
<!-- SECTION:NOTES:END -->
