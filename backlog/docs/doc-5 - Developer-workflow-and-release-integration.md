---
id: doc-5
title: Developer workflow and release integration
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 15:28'
---
# Reproducibility

Pixi is the intended task entrypoint and lock authority for developer tools; Cargo.lock records the repository build graph. pixi-sandbox carries pinned environments and, when configured and verified, vendored Cargo dependencies. Fresh-clone and restored/offline gates must be tested, not inferred.

# Quality ownership

Alint validates declarative structure and generated-workflow policy. Python scripts and their pytest suite cover Clifacet-specific semantic checks. Cargo, rustfmt, Clippy, nextest, doctests, trybuild, Taplo, actionlint, cargo-deny and cargo-audit remain separate tools. CI and hooks call canonical Pixi tasks.

# Release

The user reports crates.io trusted publishing configured for this GitHub repo's `release.yml` and environment `crates`. The release publishes the macro crate before the main crate, waiting for availability. The release job requests only required permissions and exchanges OIDC for a short-lived token. Validate the registry bindings and exact commit, preserve resumability and require authorization before live publication.

pixi-release's inspected owner policy demonstrates GitHub Release and Prefix targets. Whether and how it can generate/orchestrate this crates.io sequence is a milestone-0 investigation; use a documented supported integration or propose upstream support, not a manually patched generated workflow.

# Compiler

Edition 2024 and a Pixi-pinned stable compiler are intended. MSRV 1.88 is provisional until all supported feature/dependency combinations compile in CI (decision-10).
