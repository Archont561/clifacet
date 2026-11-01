# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
`scripts/bump_version.py` promotes the `Unreleased` section when a release is
prepared, so the headings below are managed by that script.

## [Unreleased]

### Added

- `#[clifacet::command(input = …)]`, which wraps `clap`'s parser and the input bridge in
  one attribute instead of two that have to agree. It expands to the item exactly as
  written, preceded by `#[derive(::clap::Parser, ::clifacet::Facet)]` and `#[facet(…)]`,
  so the wrapper emits the derive rather than a second implementation and the two cannot
  drift. `clap` still has to be a direct dependency, because `clap`'s own derive expands
  to code naming `clap::…`. `#[derive(clap::Parser, clifacet::Facet)]` is unchanged and
  still works.
- `Policy::prefer(Mode)`, which separates what an application *permits* from what it
  *wants*: the ceiling still bounds everything, and the preference decides the mode used
  when the user asked for nothing, without ever falling back to the ceiling when one is
  stated. `Policy::preference()` reports it and `Policy::default_mode()` resolves it, so
  a run that asks nothing is explainable in one call. A preference above the ceiling
  cannot be written: `prefer` checks it in a `const fn`, so a `const` policy asks for
  too much at compile time instead of being silently clamped. Without a preference, a
  policy behaves exactly as before — the ceiling is the default.

### Changed

- The reference application moved from `crates/clifacet-example` to
  [`examples/simple`](examples/simple/README.md) and gained the README it was missing:
  what to run, the flags, the four modes, the streams, the exit codes and which test
  proves each promise. The package name is unchanged, so `cargo run -p clifacet-example`
  and every documented path still work; the workspace member, the Alint rules, the
  hook globs, the `check-docs` paths and the guides follow the new location.
- The example's policy is now `Policy::up_to(Mode::Tui).prefer(Mode::Interactive)`: a
  run with no flags asks for what the flags left out, and `--tui` is what opens the
  form. Permissions are unchanged — every mode is still available to it.
- The release and CI workflows are plumbing now. Every decision they used to make in shell
  lives in a script with tests that need neither a registry nor a runner: `scripts/release.py`
  (`preflight`, `package`, `report`, `publish`, `tag`), `scripts/commit_range.py`,
  `scripts/advisory_cache.py`, and `scripts/github_actions.py` for the two Actions file
  protocols. `release.yml` shrank from 216 lines to 90 and no longer names a crate, an index
  URL or `cargo publish`; a test fails when a `run:` block grows a shell condition, a loop or
  a command substitution. A crate whose index entry cannot be read is reported as
  unreadable rather than as absent, which is the one thing it must not be mistaken for.

## [0.1.0] - 2026-11-01

### Added

- Guides for the implemented API: [getting started](docs/getting-started.md),
  [modes and the interaction policy](docs/modes.md), [the companion
  derive](docs/derive.md), [TUI composition](docs/tui-composition.md),
  [testing](docs/testing.md) and [contributing](docs/contributing.md), with a rewritten
  root README and docs index that point at them. `docs/command-core.md` stays the
  reference.
- `scripts/check_docs.py` and the `check-docs` task, which make the guides part of the
  build: relative links and `#fragment` anchors must resolve, every ```` ```rust ````
  fence is compiled into `examples/simple/tests/documented_snippets.rs` and run
  by the suite (a `rust,ignore` fence is an illustration), every documented
  `clifacet::…` path is checked segment by segment against what the crate declares, and
  the reference application is held to staying domain-neutral.
- `scripts/check_advisory_exception.py` and the `check-advisory` task, which give the
  RUSTSEC-2026-0374 exception an expiry. It renders the dependency path from
  `Cargo.lock` — currently `rstest-bdd 0.6.0 → i18n-embed 0.16.0 → i18n-embed-impl
  0.8.4 → i18n-config 0.4.8 → basic-toml 0.1.10` — and fails the build when the
  unmaintained crate is gone (so both ID-wide exceptions must be deleted), when either
  `deny.toml` or `.cargo/audit.toml` no longer carries the exception, or when
  `docs/development.md` quotes a path the lockfile no longer agrees with.
- `scripts/check_packages.py` and the `check-packages` task, which check what
  `cargo publish` would upload without publishing anything: both crates must package
  their manifest, source root, README and licence texts, must not package repository-only
  paths such as BDD `.feature` files or `.pixi-sandbox/`, and must declare the metadata a
  crates.io page shows. Writing it found that neither published crate shipped the licence
  texts at all — they were only at the repository root — so each crate now carries
  `LICENSE-MIT` and `LICENSE-APACHE`, checked byte for byte against the root copies.
- A qualified compatibility matrix instead of a claimed one. `scripts/check_features.py`
  derives the supported feature combinations from the crate manifest — the empty set, the
  `default` set, each frontend alone and all of them together — and CI checks each on
  stable; `scripts/prove_msrv.py` now reuses that same matrix, so the stable rows and the
  Rust 1.88 floor cannot drift. A new `platforms` CI job runs the headless suite on Linux,
  macOS and Windows, and `docs/development.md` states what is and is not covered.
- The reference application is now a workspace crate of its own, `clifacet-example`,
  with its tests, BDD features, proptest seeds and runnable demos beside it. It depends
  on `clifacet` and on nothing private, so it is a proof of the published API rather
  than part of the crate it demonstrates. Run it with
  `cargo run -p clifacet-example -- --name ada`, and the demos with
  `cargo run -p clifacet-example --example parity_report`.

- `clifacet::contract::ExitPolicy`, the documented mapping from outcome to exit code.
  `ExitPolicy::standard()` is the conventional one (0, 2, 130, 1) and the reference
  application now names it `clifacet_example::EXIT`, so the constants and the policy cannot
  drift. The four outcomes a caller distinguishes are pairwise distinct; a usage error
  shares validation's code, which is asserted so the overlap is a decision.

### Removed

- `clifacet::example` is gone from the library. The example was never part of the
  library's promise — it is an application — and shipping it meant publishing a
  reference application inside a package whose API it was supposed to demonstrate.
  Everything it used was already public; only the `example` module itself was not.
  Import `clifacet_example` (the `clifacet-example` crate) instead.

### Changed

- `clifacet_example::POLICY` is `Policy::up_to(Mode::Tui)`. The example crate enables
  the `tui` frontend unconditionally, so a mode it documents is always a mode it can
  host; an application that enables fewer frontends still sets a lower ceiling.

### Fixed

- `test-local` — the offline gate every guide and `AGENTS.md` tell a contributor to run
  with `--no-install` after a restore — depended on `deny`, which fetches the RustSec
  database. On a restored, network-free checkout it failed for a reason that had nothing
  to do with the change being checked. It now uses `deny-offline` and also runs the
  doctests, and `scripts/tests/test_pixi_tasks.py` asserts the gate stays offline-safe.
- The `crates` environment was documented as having "no protection rules". It has one: a
  branch policy restricted to `main`. The claim now distinguishes that scope gate from the
  approval gate that is still missing (required reviewers), and records the query that
  establishes it.
- A full-screen run could block forever when its stdin was not a terminal. `live_keys`
  called `crossterm::event::read()` unconditionally, so a process whose keyboard could
  never produce an event — a pipe, a test runner, a Windows console handle that exists
  but is not interactive — waited instead of noticing. It now ends its input when stdin
  is not a terminal, which `run_with` already reports as a cancellation. Found by the new
  Windows CI job (CLI-16), which is the only place the two behaviours differ.
- A full-screen run drew its screen on `stdout`, putting escape sequences in the data
  stream a caller asked for, and bypassed the diagnostic stream the application was
  given. The form now draws on that stream — the same one prompts and diagnostics use
  — so `stdout` carries only data in every mode. `Capabilities::detected` requires a
  terminal on both streams for a TUI, because a piped data stream asks for data rather
  than for a screen.
- The prompt's question was only asserted through a scripted frontend, which answers
  without writing anything; it is now covered with a real `Terminal` writing into a
  buffer the test owns.
- Framework-owned secret handling: `clifacet::secret::Secret<T>` redacts itself in
  `Debug` and `Display`, so every place the framework formats a value — a diagnostic,
  a parity harness record, a form field, a failing test's message — shows `[redacted]`
  instead. `Secret::expose` is the only read. The example gained a `--token` flag and
  refuses a credential beginning with `revoked`, which is what a redacted diagnostic
  looks like end to end.
- `clifacet::provenance`, the record of where each input value came from: `Source`
  (`argv`, `environment`, `default`, `prompt`, `form`), `Origin` and `Provenance`.
  Entries hold a name, a source and whether the value is secret — there is no field
  for a value, so the record cannot leak one.
- A secret form field: `FieldState::mark_secret` (and `FormState::mark_secret`) makes
  a preview draw `[redacted]` without changing what the form submits.
- `clifacet::harness`, the cross-mode parity harness: `Outcome` reduces a run to the
  four subjects two runs can disagree about, `Recording` wraps a `Command` and records
  the input each execution received (failures included), and `Parity` compares runs
  and names the mode and subject that disagreed. It is public API, exercised by the
  crate's tests and by `examples/parity_report.rs`, which can only use what a consumer
  can use. The example's entry points gained `main_with_command`, `run_with_command`,
  `tui_submit_with` and `run_tui_with`, so a command can be observed without a
  test-only copy of the pipeline.
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

- The release workflow resolves the changelog notes before the first publish, so a
  version with no readable section fails with nothing to revert instead of failing at
  the tag step after both crates are on the registry. The tag step reuses the file
  that was verified.
- A first release can date its changelog: `bump-version --promote-current` promotes
  the `Unreleased` section at the recorded version. Every other path refused it,
  because a version already recorded in the manifests is not a bump.

### Known limitations

- The `json` feature selects `serde_json`, but no flag produces JSON yet.
- Redaction covers the framework's own output. Application-owned logs, a credential
  passed on the command line (visible in `ps` and shell history), an environment
  variable, and a prompted answer the terminal echoes are documented boundaries, not
  guarantees; a credential held as a plain `String` is not protected at all.
- The `crates` GitHub environment restricts deployments to `main` (a branch policy) but
  has no required reviewers, so nothing asks a person before a run publishes. Release
  authorization rests on the manual dispatch and the maintainer who made it.

[Unreleased]: https://github.com/Archont561/clifacet/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Archont561/clifacet/releases/tag/v0.1.0
