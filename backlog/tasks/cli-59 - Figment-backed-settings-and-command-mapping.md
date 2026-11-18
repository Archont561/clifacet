---
id: CLI-59
title: 'Figment-backed configuration engine and command function mapping'
status: Done
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
- [x] #1 An optional Cargo feature `settings-figment` provides a Figment-backed settings loader that maps nested command paths (`serve`, `db.migrate`) to Figment dictionary tables
- [x] #2 Precedence follows flag > environment > Figment profile/layers > built-in default; CLI flags override Figment values, and Clap's value parsers validate injected defaults
- [x] #3 `#[clifacet::command]` on a function derives `serde::Deserialize` and `serde::Serialize` on the generated `*Input` struct when the `serde` feature is active, enabling direct `figment.extract_inner::<Input>(command)`
- [x] #4 Key normalization transparently resolves kebab-case CLI flags and snake_case config keys (e.g. `listen-port` <-> `listen_port`)
- [x] #5 Provenance correctly records `Source::File` with path attribution when values are supplied by Figment file providers
- [x] #6 Interactive prompts and TUI forms receive Figment-configured defaults as initial values
- [x] #7 The dependency graph and licenses for Figment and its transitives (`uncased`) pass `cargo deny` and satisfy repository audit policy
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented decision-22 hybrid configuration model connecting Figment and clifacet:

**Delivered Components:**
- `crates/clifacet/src/settings/figment.rs`: Implemented `FigmentLoader` and `flatten` converting Figment dictionary trees into clifacet `Entry` items, preserving file origin metadata through `figment.find_metadata(...)`.
- `crates/clifacet/src/settings.rs`: Added `Settings::from_figment` and key/section normalization (`normalize_key`, `section_matches`, `section_starts_with`) supporting seamless interoperability between `kebab-case` CLI arguments (`listen-port`) and `snake_case` configuration keys (`listen_port`), while rejecting duplicates.
- `crates/clifacet/src/app.rs`: Implemented `App::figment(self, figment: figment::Figment) -> Self` runner integration.
- `crates/clifacet-macros/src/function.rs`: Added `#[cfg_attr(feature = "serde", derive(::serde::Deserialize, ::serde::Serialize))]` to generated command input structs (`*Input`) and kebab-case field aliases `#[cfg_attr(feature = "serde", serde(alias = #kebab))]`.
- `crates/clifacet/src/secret.rs`: Implemented `serde::Deserialize` for `Secret<T>` so redacted secrets can be loaded from configuration.
- `crates/clifacet/tests/settings_figment.rs`: Comprehensive integration test suite verifying nested command mapping (`serve`, `db.migrate`), precedence (`flag > env > figment > default`), Clap value parser validation, Serde extraction, key normalization, provenance tracking with file attribution, interactive prompts and TUI form pre-population, and Figment profile switching.
<!-- SECTION:NOTES:END -->
