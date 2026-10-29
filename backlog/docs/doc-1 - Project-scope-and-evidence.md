---
id: doc-1
title: Project scope and evidence
type: other
created_date: '2026-10-29 15:27'
updated_date: '2026-10-29 15:28'
---
# Purpose and status

Clifacet is a proposed Rust library for clap-based commands with non-interactive, interactive and TUI presentation. One command core and a shared validation/execution path are the central boundary (decision-1). `clifacet` and `clifacet-macros` exist at 0.0.0; no library behavior is implemented in this planning scaffold.

# In scope for 0.1.0

- A clap-first companion derive, subject to compatibility testing (decision-2).
- Deterministic interaction policy, prompts and a minimal composable ratatui form.
- Shared validation, application-owned output/exit policy, secret-safe framework output and a public cross-mode test harness.
- The developer, quality and release foundations in decisions 5–10.

# Deferred unless reprioritized

Plan/confirm/dry-run lifecycle, machine-readable command contract export, bounded event/diagnostic sinks, complex choice providers, arbitrary wizard graphs, global registration and universal UI backends.

# Evidence and honesty

Static source audit of Archont561 repositories and upstream libraries found useful patterns in pixi-release, pixi-sandbox, Universe, qgis-sdk, bun-test-utils and staged-recipes-submit. Python/TypeScript repos are pattern donors, not migration targets. Universe is a potential pilot only when a real command warrants it. Source inspection is not a successful build. `clap-tui` already offers CLI-to-TUI generation, clap revalidation and an internal scripted TUI harness; do not claim those concepts as novel.

Sources: [pixi-release](https://github.com/Archont561/pixi-release), [pixi-sandbox](https://github.com/Archont561/pixi-sandbox), [Universe](https://github.com/Archont561/universe), [clap-tui](https://github.com/gitseti/clap-tui).
