# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
`scripts/bump_version.py` promotes the `Unreleased` section when a release is
prepared, so the headings below are managed by that script.

## [Unreleased]

### Added

- Two explicit escape hatches for the full-screen form: `tui::CustomField` lets an
  application draw one answer with its own widget, and `app::run_with` (with
  `run_on_with` for the alternate screen) replaces the whole drawing while keeping
  the same keys, refusals, submission and handler. Neither is a registry or a
  backend abstraction, and `examples/tui_custom_screen.rs` shows both at once.
- A full-screen form built from public `ratatui` components: `clifacet::tui` exposes
  the state machine (`FormState`, `Key`, `Outcome`), the widgets (`Form`, `Field`)
  and the loop (`app::run`), and the generated form is assembled from those same
  widgets, so an application can reuse them for its own screen. Submission becomes
  arguments and goes through `clap` and the derived validation, so a typed answer
  produces exactly what the equivalent flags produce; a refused answer stays on the
  form with the same wording. `--tui` is honoured when the `tui` feature is built
  and refused with a message when it is not.
- A prompt adapter on the same command core: `clifacet::prompt` collects what flags
  left out through a `Prompter`, asking only when the resolved plan permits and never
  re-asking a supplied value. `Terminal` asks a person, `Scripted` answers from a
  list for tests, and `Session::run` makes collection-before-execution structural.
  The example's policy now allows prompts, so `--interactive` works and `--yes`
  answers its confirmation without a question.
- A deterministic interaction policy: `clifacet::policy` resolves the runtime mode
  from an explicit request, then the application's policy, then the environment,
  with a plan that is the only route to a frontend. Refusals explain themselves and
  execute nothing, `--yes` affects confirmations only, and cancellation is its own
  `FailureKind` with its own exit code in the example.
- Case tables, properties and their documented limits: the command core's three
  layers (draft, input, validation) are covered by `rstest` fixtures, four
  invariants by `proptest` with committed regression seeds, and `docs/testing.md`
  records the installed versions, the proven MSRV scope and the coverage rule.
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

### Fixed

- The release workflow resolves the changelog notes before the first publish, so a
  version with no readable section fails with nothing to revert instead of failing at
  the tag step after both crates are on the registry. The tag step reuses the file
  that was verified.
- A first release can date its changelog: `bump-version --promote-current` promotes
  the `Unreleased` section at the recorded version. Every other path refused it,
  because a version already recorded in the manifests is not a bump.

### Known limitations

- The `json` feature selects `serde_json`, but no flag produces JSON yet.
- The `crates` GitHub environment has no protection rules yet, so release
  authorization rests on the manual dispatch alone.

[Unreleased]: https://github.com/Archont561/clifacet/commits/main
