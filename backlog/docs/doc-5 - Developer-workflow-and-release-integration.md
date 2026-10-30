---
id: doc-5
title: Developer workflow and release integration
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 18:20'
---
# Reproducibility

Pixi is the intended task entrypoint and lock authority for developer tools; Cargo.lock records the repository build graph. pixi-sandbox carries pinned environments and, when configured and verified, vendored Cargo dependencies. Fresh-clone and restored/offline gates must be tested, not inferred.

# Quality ownership

Alint validates declarative structure and generated-workflow policy. Python scripts and their pytest suite (`pixi run --frozen test-scripts`) cover Clifacet-specific semantic checks; pytest and alint are pinned PyPI dependencies, so the sandbox packs both wheels. Cargo, rustfmt, Clippy, nextest, doctests, trybuild, Taplo, actionlint, cargo-deny and cargo-audit remain separate tools. CI and hooks call canonical Pixi tasks.

# Release

The user reports crates.io trusted publishing configured for this GitHub repo's `release.yml` and environment `crates`. The release publishes the macro crate before the main crate, waiting for availability. The release job requests only required permissions and exchanges OIDC for a short-lived token. Validate the registry bindings and exact commit, preserve resumability and require authorization before live publication.

pixi-release's inspected owner policy demonstrates GitHub Release and Prefix targets. Whether and how it can generate/orchestrate this crates.io sequence is a milestone-0 investigation; use a documented supported integration or propose upstream support, not a manually patched generated workflow.

# Compiler

Edition 2024 and a Pixi-pinned stable compiler are intended. MSRV 1.88 is provisional until all supported feature/dependency combinations compile in CI (decision-10).

# Verified state

Checked 2026-10-29 against `main` at `3930c68`, not inferred from configuration:

- CI (repository/Rust quality and tests/coverage) passes end to end on `main`, including alint, the skills and Backlog checks, the feature matrix, `cargo fmt`, Clippy with `-D warnings`, `cargo deny`, `cargo audit` and the Codecov upload.
- The sandbox publisher runs unattended on `main` and republishes `sandbox/developer-linux-64`. A fresh clone restored `34a373a` with no network: `doctor --verify` matched all 14,869 declared blobs, and `fmt-check`, nextest and Clippy completed against the vendored crates.
- `alint` is pinned to `==0.20.1` in `[pypi-dependencies]` because `.alint-baseline.json` records that version and the sandbox packs that single wheel.
- Known gap: `pixi install --frozen --offline` is not a no-op after a restore. It removes the PyPI-installed `alint` and then fails, since Pixi's own cache never receives the wheel; the supported offline path is `pixi run --no-install --offline`. Fixing this belongs upstream in pixi-sandbox.
- MSRV 1.88 remains unproven: no CI job compiles at 1.88 (CLI-16).

