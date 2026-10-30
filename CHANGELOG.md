# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
`scripts/bump_version.py` promotes the `Unreleased` section when a release is
prepared, so the headings below are managed by that script.

## [Unreleased]

### Added

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
- Git hooks through lefthook: linters, formatters and repository checks before
  a commit, advisories and tests before a push, and convco validating the commit
  message against the `.versionrc` configuration.

### Known limitations

- The library exposes only `VERSION` and `version!()`; the command, interactive
  and TUI APIs are planned in milestones M1 to M3.
- The `crates` GitHub environment has no protection rules yet, so release
  authorization rests on the manual dispatch alone.

[Unreleased]: https://github.com/Archont561/clifacet/commits/main
