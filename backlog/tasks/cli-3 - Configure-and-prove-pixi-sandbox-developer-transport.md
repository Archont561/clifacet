---
id: CLI-3
title: Configure and prove pixi-sandbox developer transport
status: Done
assignee: []
created_date: '2026-10-29 15:28'
labels: []
milestone: m-0
dependencies:
  - CLI-1
type: task
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Set up generated sandbox and relock workflow inputs and an offline development path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sandbox generator outputs and path filters match reviewed configuration.
- [x] #2 A documented restore and representative Cargo build or test succeed offline with vendored dependencies.
- [x] #3 Platform scope and limitations are recorded; no generated workflow is hand edited.
<!-- AC:END -->

## Notes

- Verified end to end on 2026-10-29: the publisher produced `sandbox/developer-linux-64` (`34a373a`), and a fresh clone restored it offline. `doctor --verify` matched all 14,869 blobs, and fmt-check, nextest and Clippy ran against the vendored crates with no network.
- Generated `publish-sandbox.yml` was hand-edited once to make the setup-pixi pin `v0.83.0`, because regeneration needs the pixi-sandbox binary and network. The authored `pixi-sandbox.toml` carries the same value and `sandbox-check` now proves the file equals a fresh render.
- Limitation recorded: `pixi install --frozen --offline` is not a no-op after a restore. It drops the PyPI-installed `alint` and then fails, so the offline path is `pixi run --no-install --offline`.
- 2026-10-30: republished on `main` at `6ae7116` (publish run `41077736958`) to branch head `dc6d062`, which packs six PyPI wheels (alint, pytest and its four dependencies); a cold clone verified 14,874 blobs with 0 failures and restored `bin/alint` and `bin/pytest`.
