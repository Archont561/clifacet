# Contributing

## Start a session

```sh
bash scripts/bootstrap.sh
export PATH="$HOME/.local/bin:$PATH"
```

`bootstrap.sh` fast-forwards a clean checkout to its upstream, restores the sandbox
environment when Pixi is missing, installs the Git hooks and ends with a smoke check.
To restore without the checkout step, run the generated `scripts/restore.sh` directly.
The launcher may replace the local `.pixi` environment; on a shared host, read it
before running it.

If a restore has just happened, run tasks with `--no-install`: the environment is
already the one the lockfile describes, and re-solving it needs the network.

## Use the task surface

```sh
pixi run --frozen --no-install test-local   # the loop's gate: fmt, clippy, alint, tests, scripts, deny, sandbox
pixi run --frozen quality                   # everything CI's quality job runs, plus Backlog and advisory checks
pixi run --frozen test                      # both nextest groups
pixi run --frozen coverage                  # LCOV plus the 90% source-line gate
```

`quality` fetches Bun packages and the RustSec database, so it is a CI command rather
than a fast inner loop. Use `pixi run --frozen -x cargo …` for a single Cargo command
where no project task exists.

## What a change has to bring

Every feature starts by reading three skills — `.agents/skills/tdd`, `bdd` and
`clean-code-principles` — **before** any production code is written, and the task
records the evidence: the failing test that came first, the scenario that describes
the behaviour, and the design decision that kept the change proportionate.

Two narrowings exist and both are stated rather than assumed: a change with no
user-visible workflow says so and explains why a scenario would not describe anything
real, and a mechanical change with no behaviour — a rename, a dependency bump, a docs
or CI edit — may take the exception. A red test is never turned into a green one by
weakening it.

## Documentation is checked

The guides are part of the build, not decoration:

```sh
pixi run --frozen check-docs   # links, anchors, compiled snippets, documented names
```

- Every relative link has to resolve, and every `#fragment` to a heading that exists.
- A ```` ```rust ```` fence is **compiled**: `scripts/check_docs.py` writes it into
  `crates/clifacet-example/tests/documented_snippets.rs`, which the suite builds. A
  fence tagged `rust,ignore` is an illustration and is not compiled — use it when the
  code is deliberately a fragment.
- Every `clifacet::…` path is checked segment by segment, so a guide cannot teach a
  function the crate does not have.
- The reference application stays domain-neutral: a page that dresses it as a product
  fails the check.

After editing a page, regenerate the snippet file and commit it:

```sh
python scripts/check_docs.py --write
```

## Git hooks and commits

| Hook | Runs |
| --- | --- |
| `pre-commit` | `alint`, `fmt`, `lint`, `check-features`, `actionlint`, `sandbox-check`, `skills-check` |
| `pre-push` | `audit-offline`, `deny-offline`, `test`, `test-doc`, `test-scripts` |
| `commit-msg` | `convco check --from-stdin --strip` |

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
`.versionrc` maps each type to a Keep a Changelog section, so `convco changelog` and
`scripts/bump_version.py` agree on headings — a message CI cannot read is a message
the changelog cannot use.

## Where things live

| Path | What it holds |
| --- | --- |
| `crates/clifacet/src` | the library: the command seam, the frontends, the policy, the contract |
| `crates/clifacet-macros` | the `Facet` derive |
| `crates/clifacet-example` | the reference application, its tests and runnable demos — not published |
| `docs/` | these guides |
| `backlog/` | decisions, milestones and tasks, as the source of direction |
| `scripts/` | the repository checks, each with a pytest suite in `scripts/tests/` |

Generated files are never edited by hand: `scripts/restore.sh`, the sandbox workflows
and the documented-snippet file are all rendered from their source of truth.
`pixi run --frozen sandbox-check` proves the first two match.

## Release

Publishing is manual and authorized: see
[Release](development.md#release). Nothing in a normal contribution publishes
anything.
