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
- 2026-10-30: the relock bot repaired `pixi.lock` for `convco` after a manifest edit made where the conda channels are unreachable (`5eda2272`, 16 added lines, no other package moved). Two observations. (a) The bot's `relock.yml` renders `setup-pixi` at `pixi-version: v0.81.0` while `ci.yml`, `publish-sandbox.yml` and `release.yml` all pin `v0.83.0`, so its output is not byte-identical to a local `pixi lock`: this repair dropped `attestations_sha256` from the `pixi-sandbox` entry. That field is provenance metadata, not a resolution difference, so the publisher still installs the tree; but the lock now records less than a local relock would. (b) `relock.yml` is generator-owned, so the pin cannot be fixed without a pixi-sandbox release that renders the configured `pixi_version` for it too — recording it here rather than hand editing a generated file.
- 2026-10-30: after `shellcheck` was added to `pixi.toml` and the relock bot repaired `pixi.lock` (`39c268ae`), the publisher packed the package but not its files. Evidence: the sandbox branch published from that same commit lists `pack/channel/linux-64/shellcheck-0.11.0-hfc2019e_2.conda` in the manifest's blobs, while its environment file list (11,432 entries) contains no `shellcheck` path at all and no `conda-meta/shellcheck-*.json`; a restore therefore yields an environment without the binary. `convco`, added the same way an hour earlier, *is* present, so either the first publish after a lock change packs the archive ahead of the environment, or the pipeline installs from a restored environment rather than from the checked-out lock. Worth one more publish cycle to tell those apart, and worth reporting upstream if it repeats: the workaround is to re-publish after merging rather than relying on the publish that follows the relock commit.
