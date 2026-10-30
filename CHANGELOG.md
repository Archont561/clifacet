# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
`scripts/bump_version.py` promotes the `Unreleased` section when a release is
prepared, so the headings below are managed by that script.

## [Unreleased]

### Added

- `clifacet::Facet`, the companion derive: it connects an existing `clap`
  command to validated typed input without a second grammar, reports `clap`'s own
  command (aliases, defaults and value parsers included), and builds the input
  either by mirroring fields or through a declared bridge. Unsupported shapes and
  attribute mistakes are compile errors with `trybuild` golden files. The
  reference application now demonstrates an alias, a clap default, a custom
  value parser and a scenario-driven test.
- A UI-free command core: the `Command` seam, a library-owned `Failure` with
  validation and execution categories, and a reference application
  (`clifacet::example`) that keeps parsing, validation, execution and
  presentation in separate layers. Applications own the streams and the exit
  codes; the library owns the failure kinds.
- Rust workspace with the `clifacet` library and the `clifacet-macros`
  procedural-macro crate, edition 2024, workspace lint policy and a declared
  minimum supported Rust version of 1.88 that CI proves.
- Reproducible Pixi task surface, offline sandbox transport through
  pixi-sandbox, and a dev container for VS Code and Codespaces.
- Repository policy through alint, plus tested Python checks for coverage and
  the agent skills, all run by pytest.
- CI, lockfile relock and sandbox publication workflows, with Codecov coverage
  reporting.
- `release.yml` for crates.io trusted publishing: manual dispatch, dry run by
  default, macro-first ordering and a publish step that skips versions already
  indexed.
- Agent operating model, project backlog and local skills.
- Release lifecycle tooling: `scripts/bump_version.py` bumps every recorded
  version site and promotes the changelog, and `release.yml` tags the release
  with those notes once both crates publish.
- `scripts/bootstrap.sh` also forces the sandbox branch refresh a restore needs,
  since a published branch is rewritten and the generated launcher never
  re-fetches it.
- `scripts/bootstrap.sh`, which brings any checkout to a working session in one
  command: fast-forward to the remote head without touching local work, restore
  the sandbox environment, install the hooks, and smoke-check the result.
- `shellcheck` is pinned in the Pixi environment, so the local `actionlint`
  task also checks workflow shell scripts, exactly as CI does.
- Conventional commit messages are enforced in CI, and the advisory gates work
  offline: `audit-offline` audits against a local RustSec clone or steps aside
  with a message, while CI keeps running the fetching audit.
- Git hooks through lefthook: linters, formatters and repository checks before
  a commit, advisories and tests before a push, and convco validating the commit
  message against the `.versionrc` configuration.

### Known limitations

- The command core has no interactive or TUI frontend, no `--json` output and no
  runtime mode resolution yet; those are milestones M2 and M3.
- The `crates` GitHub environment has no protection rules yet, so release
  authorization rests on the manual dispatch alone.

[Unreleased]: https://github.com/Archont561/clifacet/commits/main
