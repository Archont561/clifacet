# Development

The workspace is an early scaffold, not the released `0.1.0` library. The
checked-in Pixi environment currently targets Linux x86-64.

```sh
pixi install
pixi run hooks-install  # in a Git checkout
pixi run quality        # repo, skills, Backlog, features, sandbox, Rust and advisory checks
pixi run check-features # base and each optional dependency boundary
pixi run test-src       # crate-internal src/ tests
pixi run test-integration # external tests/ targets, including compile-fail cases
pixi run test           # both nextest groups
pixi run test-doc       # doctests (not run by nextest)
pixi run test-scripts   # repository script tests
pixi run coverage       # LCOV report plus a 90% source-line gate
```

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
credentials are stored in this repository. The setup does not install Git hooks,
commit, push or release.

This online bootstrap is distinct from the Git-branch sandbox restore described
below. It is not an offline image, and it neither publishes nor fetches the
sandbox branch automatically. Re-running `.devcontainer/setup.sh` installs or
rechecks the tools and can use the network again.

## Rust dependencies and features

The root Cargo manifest centralizes dependency versions. The main crate includes
clap, serde, thiserror and the companion macro crate as core dependencies. It
also reserves three **optional dependency boundaries**, disabled by default:

- `interactive`: inquire with the crossterm backend.
- `tui`: ratatui and crossterm (the same crossterm 0.29 line as inquire).
- `json`: serde_json.

These features currently only select dependencies; the planned command, prompt,
TUI, JSON and derive APIs are **not implemented**. The macros crate includes
proc-macro2, quote and syn for the planned companion derive. rstest, rstest-bdd,
proptest, assert_cmd and trybuild remain development dependencies. The selected
inquire adapter is a starting dependency, not a promise to expose its internal
backend for testing; the planned prompt harness remains framework-owned.

`Cargo.lock` resolves all declared feature dependencies, including optional
ones. `cargo fetch --locked` (through `pixi run --frozen -x`) downloads this
locked graph when connected; the sandbox's Cargo vendoring can carry that graph
after a new sandbox is published. A checked-in manifest or lock alone does not
make a fresh machine offline-ready, and future dependency changes require a
refreshed lock and republished sandbox. A local linux-64 transport was packed,
verified and restored, then checked offline with all targets and features; live
GitHub publication has not been tested. `check-features` checks the base and
individual features; Clippy and test tasks also use `--all-features`. It does not
prove every feature combination or platform. The locked Linux graph was checked
with Rust 1.88, but a full MSRV and operating-system test matrix is future work.

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
wheel for `alint`. It is packed under
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
consumer-facing integration and compile-fail tests live under `tests/`.
Procedural macro crates cannot expose a normal public constant.

The LCOV gate checks executable lines in both crates' `src/` directories and
fails if either crate has no source coverage. It merges repeated line records
because procedural macros run during compilation in multiple processes. It
also reconciles LCOV's line summaries with those merged line hits before upload.
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
