# Development

The workspace is an early scaffold, not the released `0.1.0` library. The
checked-in Pixi environment currently targets Linux x86-64.

```sh
bash scripts/bootstrap.sh  # checkout at the remote head, environment, hooks, smoke check
pixi run quality        # repo, skills, Backlog, features, sandbox, Rust and advisory checks
pixi run check-features # every supported feature combination, one cargo check each
pixi run test-src       # crate-internal src/ tests
pixi run test-integration # external tests/ targets, including compile-fail cases
pixi run test           # both nextest groups
pixi run test-doc       # doctests (not run by nextest)
pixi run test-scripts   # repository script tests (pytest over scripts/tests)
pixi run coverage       # LCOV report plus a 90% source-line gate
```

`quality` fetches Bun packages and the RustSec advisory database, so it is a CI
command rather than a fast inner loop. Before each commit, run the offline subset
through Pixi with `--no-install` (a restore already materialised the environment):

```sh
pixi run --frozen --no-install test-local
```

That covers formatting, Clippy, alint, both nextest groups, the script tests,
`cargo deny` and the sandbox check in about a minute on a warm checkout.

## Command core

The UI-free command seam lives in `crates/clifacet/src/{command,failure}.rs`, the
frontends in `crates/clifacet/src/{prompt,policy,tui}`, secrets and their provenance in
`crates/clifacet/src/{secret,provenance}.rs`, and `docs/command-core.md` records the four
layers, the ownership rules and what is deliberately not implemented yet. The reference
application is a crate of its own, `crates/clifacet-example`, which depends on `clifacet`
and on nothing private: it is the proof that the published API is enough. Run it against
the real streams with:

```sh
pixi run --frozen -x cargo run -p clifacet-example -- --name ada --repeat 2
```

Its tests pass their own buffers instead of a terminal, so `test-integration` covers the success,
validation, execution, usage and help paths without a TTY.

The full-screen form needs a real terminal to be typed into; the example crate enables
the library's `tui` feature:

```sh
pixi run --frozen -x cargo run -p clifacet-example -- --tui
```

Its tests need neither: `tui::app::run` takes a `ratatui` backend and a list of keys, so
`TestBackend` plus a script covers drawing, editing, submission and cancellation. `main_with` with
a `tui`-capable `Capabilities` covers the dispatch, and the process's own terminal is the only part
no test can reach.

## Supported platforms and feature combinations

Compatibility is a claim with a list, so the list is written down and checked rather
than assumed.

**Feature combinations.** A feature is a dependency boundary: an application enables the
frontends it uses and pays for no others. `scripts/check_features.py` derives the rows
from `crates/clifacet/Cargo.toml` and checks each one against the committed lockfile:

| Row | Flags | What it proves |
| --- | --- | --- |
| `no-default-features` | `--no-default-features -p clifacet` | nothing enabled: the base crate stands alone |
| `default` | the manifest's `default` set | what a consumer gets with a bare `clifacet` dependency |
| `interactive` | `--no-default-features --features interactive` | the prompt frontend compiles by itself |
| `tui` | `--no-default-features --features tui` | the full-screen frontend compiles by itself |
| `json` | `--no-default-features --features json` | the reserved serialization feature compiles by itself |
| `all-features` | `--all-features` (workspace-wide) | every frontend together, which is what the tests build |

`default` is empty today, so it resolves to the same configuration as
`no-default-features`; the script runs that configuration once and reports both rows, so
the table and the plan cannot disagree. Adding a feature to the manifest adds a row —
CI never carries its own list.

The same matrix is what `scripts/prove_msrv.py` checks with the declared MSRV toolchain,
so the stable matrix and the floor cannot drift: `pixi run prove-msrv` and the `msrv` job
are the same five commands with a different `cargo +<toolchain>`.

```sh
pixi run check-features                 # run the matrix
python scripts/check_features.py --print-matrix   # the rows, as data
python scripts/check_features.py --print-plan     # the commands they resolve to
```

**Operating systems.** CI runs the headless suite on Linux, macOS and Windows
(`platforms` job): `cargo test --workspace --all-features`, the documentation examples,
and a `--no-default-features` build of the library. Nothing in the workspace is
platform-specific — no `cfg(unix)`, no path handling beyond `std::path` — and no suite
needs a terminal: every frontend test drives its own buffers and keystroke scripts, so
"it needs a TTY" is never why a platform would be exempt.

Those platform jobs use the runner's own stable toolchain instead of Pixi. The checked-in
Pixi environment targets **linux-64 only**, so the jobs that ask "does this code build
here" use rustup, exactly as the `msrv` job does and for the same reason recorded in
decision-5. Pixi remains the single entry point locally and on the Linux job that owns
the repository policy checks.

**What is not claimed.** The Pixi environment is Linux x86-64; the dev container is
amd64. ARM Linux, 32-bit targets and non-tier-1 platforms are untested. Windows CI proves
the code compiles and the tests pass there, not that the full-screen interface renders
well in a Windows terminal.

## Git hooks

`pixi run hooks-install` writes the hooks `lefthook.yml` declares into `.git/hooks`. Every hook
calls a Pixi task, so a hook and the matching CI job cannot drift:

| Hook         | Runs                                                                                                 | Why there                                                                                                            |
| ------------ | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `pre-commit` | `fmt` (re-staged), `lint`, `check-features`, `alint`, `actionlint`, `sandbox-check`, `skills-check`   | Seconds on a warm checkout, and a commit is the last moment a fix is still free.                                      |
| `pre-push`   | `audit-offline`, `deny-offline`, `test`, `test-doc`, `test-scripts`                                   | Minutes of work, and exactly what the push is about to hand to CI, so they run once per push instead of per commit.  |
| `commit-msg` | `convco check --from-stdin --strip`                                                                   | The message is the input to the changelog, the version bump and the release notes.                                    |

The hooks call `pixi run --frozen --no-install`, which reuses the installed environment instead
of re-solving it: a plain `pixi run` would reach for the network and fail on an offline restore.
The advisory gates are the same story. `cargo audit` fetches the RustSec database and fails
without one, so `audit-offline` audits against a local clone when it exists and otherwise says
so and steps aside; `deny-offline` runs the three checks that need no database. CI still runs the
fetching `audit` and the full `deny`, so an offline push is never a silently unaudited one —
the gates simply live where the network does.
Run `pixi install --locked` once after cloning, and the hooks work from then on. `git commit
--no-verify` and `git push --no-verify` skip them when a work-in-progress commit genuinely needs
to land; CI remains the authoritative verdict.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/), configured
in `.versionrc` for convco. That file also maps each commit type to a Keep a Changelog section,
so `convco changelog` and `scripts/bump_version.py` agree on headings. CI enforces it too: the
`commit-messages` job runs `convco check` over the pull request's commits, so a contributor
without hooks cannot merge a message the changelog cannot read.

`actionlint` only runs its embedded shell checks when a `shellcheck` binary is on `PATH`, and
it used to find one on the CI runner but not in the Pixi environment — so a `run:` block could
pass locally and fail CI. `shellcheck` is now a pinned Pixi dependency, which removes that gap
and pins the version both places run: `pixi run actionlint` checks the workflow shell exactly
as CI does.

## Sandbox branch refreshes

Every publish rewrites `sandbox/developer-linux-64`, and the generated `restore.sh` fetches a
sandbox branch only when the ref is missing, without a force marker — so a second restore reuses
the pack it fetched the first time. `scripts/bootstrap.sh` forces the refresh before restoring;
to do it by hand:

```sh
git fetch --depth 1 origin '+refs/heads/sandbox/*:refs/remotes/origin/sandbox/*'
bash scripts/restore.sh
```

The restore reports how many entries it verified, which is the quickest way to notice that a
pack is older than the branch: it should match the manifest's file count.

## Dev container

In VS Code, choose **Dev Containers: Reopen in Container**; GitHub Codespaces
can also use `.devcontainer/devcontainer.json`. Like pixi-sandbox, it uses the
official Pixi image and a separate post-create script rather than a Dockerfile.
The setup is adapted from [pixi-sandbox's devcontainer](https://github.com/Archont561/pixi-sandbox/tree/1903b088a0896126dee82e2a8b6b007942303ffc/.devcontainer),
with project-specific locked tools and no automatic model refresh. The image is
pinned to Pixi 0.83.0 and an immutable image digest. Clifacet's
Pixi lock targets **Linux x86-64 only**; ARM hosts need an amd64 container with
working emulation. The image runs as root, so files written to a bind-mounted
workspace can be owned by root on the host.

Post-create needs network access. It installs Git, GitHub CLI and a C compiler
as Pixi global tools in the container, installs the project environment with
`pixi install --locked --all`, and installs the locked Backlog.md and skills
CLIs using `pixi run --frozen bun-install`. Git, gh and the compiler are global
bootstrap tools resolved at setup time; they are **not** covered by `pixi.lock`.
The project environment includes the pinned pixi-sandbox CLI: invoke it via
`pixi run --frozen -x pixi-sandbox`, not as an unpinned global install.

The script also installs OpenCode 1.19.5 through Bun into the container user's
global Bun directory, separately from the project `bun.lock`. To skip it, set
`CLIFACET_INSTALL_OPENCODE=0` in the host environment before creating the
container; recreate the container to change that choice. The global package is
version-pinned, but its separate global dependency graph is not in the project
lock. No provider login or model catalogue refresh is automatic; if desired,
run `opencode auth login` and `opencode models --refresh` yourself. No
credentials are stored in this repository. The setup installs the lefthook Git
hooks, but never commits, pushes or releases.

This online bootstrap is distinct from the Git-branch sandbox restore described
below. It is not an offline image, and it neither publishes nor fetches the
sandbox branch automatically. Re-running `.devcontainer/setup.sh` installs or
rechecks the tools and can use the network again.

## Minimum supported Rust version

`[workspace.package] rust-version` is the oldest compiler this repository claims
to support; every crate inherits it. The claim is verified, not assumed:

```sh
python scripts/prove_msrv.py --print-toolchain   # the declared version
python scripts/prove_msrv.py --print-plan        # the exact checks
pixi run --frozen prove-msrv                     # check with a rustup-selected toolchain
python scripts/prove_msrv.py --toolchain ""      # reuse the pinned compiler (plumbing only)
```

The `msrv` CI job installs the declared toolchain and runs that script, which
checks the empty feature set, each optional feature alone and the all-features
combination, with `--locked --all-targets`. Because the version comes from the
manifest, CI can never test a different version than the one advertised, and a
dependency that starts requiring a newer compiler fails the job with
`package X requires rustc Y`. Raising the floor is then a deliberate manifest
edit, justified by re-scanning `rust-version` across the dependency graph.

This is the one place CI does not use Pixi, since a Pixi environment pins a
single compiler (decision-5).

## Release

Publishing is manual and never triggered by a push. `.github/workflows/release.yml`
uses crates.io trusted publishing, exchanging a GitHub OIDC token for a
short-lived registry token, so no crates.io secret is stored here.

```sh
gh workflow run release.yml                     # dry run: packages the macro crate, publishes nothing
gh workflow run release.yml -f dry_run=false    # real release; see the authorization note below
```

The dispatch only runs from `main`. As of 2026-10-30 the `crates` environment
has **no protection rules**, so it does not stop a run: the manual dispatch is
the only authorization gate today. Add required reviewers to `crates` before
relying on the environment as the gate that decision-7 describes.

The notes are resolved before anything is packaged: if `CHANGELOG.md` has no dated
section for the version being released, the run stops with nothing published, and a
dry run reports it too. The tag step reuses that verified file rather than reading the
changelog a second time, so a release that has already shipped cannot fail on notes.

A dry run packages and verifies the macro crate only. The main crate cannot be
packaged before its macro version is published, so it is packaged inside the
publish loop. That loop publishes the macro crate first, waits
until that version appears in the registry index, then packages and publishes
the main crate. Both steps skip a version that is already indexed, so a run that
fails after the first publish is finished by re-running it rather than by
reverting anything.

### Cut a release

1. Describe the change in `CHANGELOG.md` under `## [Unreleased]`. The bump step
   refuses to promote an empty section, so notes cannot be skipped by accident.
2. Date that section. From the second release on, the bump in step 3 does it: it
   renames `Unreleased` to `## [<version>] - <date>` and reopens an empty one. The
   first release has nothing to bump to — the manifests already record `0.1.0` — so
   it promotes in place and leaves the version sites alone:

   ```sh
   pixi run --frozen --no-install bump-version --promote-current
   ```

   `--dry-run` previews it. Without a dated section `release.yml` refuses to start,
   because the tag step would have nothing to attach.
3. Bump every recorded version and relock in one command:

   ```sh
   pixi run --frozen --no-install bump-version 0.2.0 --relock
   ```

   That rewrites `Cargo.toml` `[workspace.package] version`, the
   `clifacet-macros` requirement in `[workspace.dependencies]`, and
   `pixi.toml` `[workspace] version`; it promotes `Unreleased` to a dated
   section; and it refreshes `Cargo.lock` offline. `--dry-run` previews the
   change and `--check` verifies the three sites agree without writing.
4. Open a PR with the bump; CI's `lock guard` proves the relock and the `msrv`
   job proves the floor.
5. Merge, then dispatch the release. The tag `v<version>` and its GitHub Release
   notes are created automatically once both crates are published:

   ```sh
   gh workflow run release.yml -f dry_run=false
   ```

## Rust dependencies and features

The root Cargo manifest centralizes dependency versions. The main crate includes
clap, serde, thiserror and the companion macro crate as core dependencies. It
also reserves three **optional dependency boundaries**, disabled by default:

- `interactive`: inquire with the crossterm backend.
- `tui`: ratatui and crossterm (the same crossterm 0.29 line as inquire).
- `json`: serde_json.

`interactive`, `tui` and the derive are implemented and covered; `json` still only
selects its dependency, and no flag produces JSON yet. Everything is checked every
way — `--no-default-features`, the empty `default` set, each boundary on its own and
all of them together — by the `check-features` task described in
[Supported platforms and feature combinations](#supported-platforms-and-feature-combinations).

The macros crate includes proc-macro2, quote and syn for the planned companion
derive. rstest, rstest-bdd, proptest, assert_cmd and trybuild remain development
dependencies. The selected inquire adapter is a starting dependency, not a promise to
expose its internal backend for testing; the planned prompt harness remains
framework-owned.

`Cargo.lock` resolves all declared feature dependencies, including optional
ones. `cargo fetch --locked` (through `pixi run --frozen -x`) downloads this
locked graph when connected; the sandbox's Cargo vendoring can carry that graph
after a new sandbox is published. A checked-in manifest or lock alone does not
make a fresh machine offline-ready, and future dependency changes require a
refreshed lock and republished sandbox. A local linux-64 transport was packed,
verified and restored, then checked offline with all targets and features; live
GitHub publication has not been tested. The locked graph is checked with Rust 1.88
across every supported feature combination and with the stable compiler on Linux,
macOS and Windows; what that does and does not cover is listed under
[Supported platforms and feature combinations](#supported-platforms-and-feature-combinations).

`cargo-deny` permits Unicode-3.0 only for locked `unicode-ident` 1.0.27: its
SPDX expression combines that Unicode data license with MIT or Apache-2.0. This
is separate from the existing RustSec advisory exception and does not change
Clifacet's own MIT OR Apache-2.0 license. Review the exception when updating the
crate; the global license allow-list remains MIT and Apache-2.0.

## Backlog and agent skills

Bun 1.4.2 is provided by Pixi. The private `package.json` pins the Backlog.md
CLI (1.53.0) and the skills CLI (1.7.4); `bun.lock` locks their transitive npm
packages and integrity hashes. Install the locked dependencies on a connected
host with `pixi run --frozen bun-install`. The `backlog` and `skills` tasks run
that frozen install first, then use `bun x --bun --no-install` to run the local
CLIs without opportunistically downloading a different CLI version. The
explicit `--bun` matters: the Node version in the Pixi environment cannot run
this version of the skills CLI. Examples (`--` forwards arguments):

```sh
pixi run --frozen backlog -- milestone list
pixi run --frozen backlog -- task list
pixi run --frozen backlog-check
pixi run --frozen skills -- list --json
pixi run --frozen skills-check
```

The **11 project-owned skills** under `.agents/skills/` are authored for this
repository and versioned with its files; they are not generated or installed
from a remote source. Their subject matter was informed by the upstream
Archont561 skill collections, but their instructions are local and adapted to
Clifacet's approval and safety rules. `AGENTS.md` requires the `session` skill
when chat history is missing or insufficient. `skills-check` validates required
skill files and basic frontmatter; script tests exercise failures. Changes to
skills are reviewed and versioned with the repository, not with a remote
`skills-lock.json`. `bun.lock` locks the **CLI package**, not remote skills.
No remote skills are auto-installed. The CLI's `add`, `update`,
`experimental_install`, and `experimental_sync` can fetch external content or
modify files and agent links: inspect the source, destination and behavior and
obtain authorization before using them. They must not overwrite local skills.

The sandbox restores Bun itself but **does not include** this project's
`node_modules` or Bun's download cache. An initial `bun-install` needs registry
access unless the exact locked packages are already cached. `pixi --offline`
alone does not force Bun offline. To prohibit network during dependency
installation, use `pixi run --frozen --offline backlog-offline -- task list` or
`pixi run --frozen --offline skills-offline -- list`; those tasks pass Bun's
`--offline` flag and fail if packages are absent from both the local install and
cache. A CLI subcommand that itself accesses a remote service can still use the
network. The local skill files can always be read without Bun or network access.
Never commit `node_modules`.

When changing `package.json`, refresh `bun.lock` on a connected host with
`pixi run --frozen -x bun install`, review both files, and validate with the
frozen task. The generated sandbox relock workflow watches both files but only
checks/repairs Pixi and Cargo locks; it does **not** repair `bun.lock`. CI's
Backlog check runs the frozen Bun install and fails on lock drift.

## Restore a sandbox

The pinned pixi-sandbox 0.9.0 configuration publishes the `default` developer
bundle for `linux-64` to `sandbox/developer-linux-64`, including vendored Cargo
sources. The generated publisher must run successfully before the first restore;
for an offline host, the sandbox branch must already be present in its Git copy.

```sh
bash scripts/restore.sh
export PATH="$HOME/.local/bin:$PATH"
pixi run --no-install --offline alint
pixi run --no-install --offline test
```

The restore already materialises the complete environment, including the PyPI
wheels for `alint` and `pytest`. They are packed under
`.pixi-sandbox/envs/default/pack/pypi/` on the sandbox branch, installed into
`.pixi/envs/default/`, and verified file by file against the manifest, so no
post-restore install is needed or possible offline. Bun-backed tasks
(`backlog-check`, and the `bun-install` prerequisites of `quality`) still follow
the rules above and are not covered by the sandbox.

Run tasks with `--no-install`. Do **not** run `pixi install --frozen --offline`
after a restore: it re-syncs the environment instead of treating it as current,
deletes the PyPI-installed files that restore wrote (leaving no `bin/alint`),
and then fails with `Failed to download alint==0.20.1 … network connectivity is
disabled`. The wheel is not in Pixi's own cache, so nothing can satisfy that
step on a disconnected host, and recovery means running the restore again.

The launcher can replace an existing `.pixi` environment and, by default,
register user tools under `~/.local/bin`. On shared machines, review its effects
first; set `PIXI_SANDBOX_USER_TOOLS=skip` to avoid changing user tools. Set
`PIXI_SANDBOX_FETCH=skip` to require a pre-fetched local branch. The launcher
must be run from a Git checkout with the branch or an `origin` remote available.

`pixi-sandbox.toml` is authored; the two sandbox workflows and `scripts/restore.sh`
are generated. After a policy change, run `pixi run --frozen -x pixi-sandbox
init --config pixi-sandbox.toml --script-path scripts/restore.sh`, then run
`pixi run --frozen sandbox-check`. The publisher force-updates the sandbox
branch; the generated relock workflow can commit lock repairs to same-repository
PR branches and dispatch validation workflows. Neither is run by ordinary tests.
The generated relock workflow pins Pixi 0.81.0; the manifest's lower bound is
compatible with that version, while normal CI uses Pixi 0.83.0.

`clifacet::VERSION` and `clifacet::version()` give the package version;
`clifacet_macros::version!()` expands to the macro package version. `clifacet`
also re-exports `version!()`. Crate-internal tests live in `src/tests.rs`;
consumer-facing integration and compile-fail tests live under `tests/`. The reference
application keeps its own suite beside it, in `crates/clifacet-example/tests/`.
Procedural macro crates cannot expose a normal public constant.

The LCOV gate checks executable lines in both published crates' `src/` directories
and fails if either crate has no source coverage. The example crate is excluded on
purpose: it is an application, it is not published, and its own suite already
exercises it end to end. The gate merges repeated line records because procedural
macros run during compilation in multiple processes, and it reconciles LCOV's line
summaries with those merged line hits before upload.
It matches the source paths and 90% threshold in `codecov.yml`. The separate
`src/tests.rs` file is excluded by cargo-llvm-cov's default test-file filter.
Since the library is a scaffold, its current coverage percentage is not
evidence of coverage of future behavior.

The CI workflow uploads coverage to Codecov. Configure the `CODECOV_TOKEN`
GitHub Actions secret if the repository's Codecov setup requires it. Making the
Codecov status required for merging is a separate GitHub branch-protection
setting; uploading alone does not configure that requirement.

## Baseline and CI caches

The checked-in `.alint-baseline.json` starts empty. It records approved existing
findings; it is **not** a cache of lint results. CI runs Alint over the tree on
every run, fails on new findings, and uses `--strict-baseline` to detect stale
entries. Never regenerate the baseline automatically in CI to hide findings.

GitHub Actions caches Cargo registry downloads and the two advisory database
clones. The advisory-cache key rotates weekly and may restore an older clone;
`cargo deny check` and `cargo audit` still fetch updates and rescan the full
lockfile every run. Cached advisory or deny *results* are never trusted, so a
new advisory can fail CI even if dependencies have not changed. The caches
contain no Cargo credentials. Pixi caches the development environment separately.

## Dependency advisory exception

`rstest-bdd` 0.6.0 brings in `basic-toml` through test-only dependencies.
RustSec flags that crate as unmaintained under
[RUSTSEC-2026-0374](https://rustsec.org/advisories/RUSTSEC-2026-0374).
The maintainer chose to keep rstest-bdd and allow this specific advisory in both
`deny.toml` and `.cargo/audit.toml`; other warnings still fail the audit. This
is an exception by advisory ID across the dependency graph, not a guarantee
that future uses of the crate are test-only. Review and remove the exception
when the dependency can be replaced or upgraded.
