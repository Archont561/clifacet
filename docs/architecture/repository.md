# How the repository is built

This page is about the repository, not the library. What the library is made of — the
command seam, the layers, the streams, the exit codes — is
[the reference](../reference/command-core.md); this page answers the other question: why
are there five crates, two bun dependencies, a Pixi environment and a Python directory,
and how do they fit together?

## One repository, four languages, one environment

| Part | Written in | What it is for |
| --- | --- | --- |
| `crates/clifacet` | Rust | the library: the command seam, the frontends, the policy, the result contract |
| `crates/clifacet-macros` | Rust | the `Facet` derive, the `#[clifacet::command]` wrapper, `version!`; re-exported so applications depend on one crate |
| `examples/` | Rust | consumers of the published API, never of a private path |
| `automation/`, `dodo.py`, `scripts/` | Python | the automation: the checks, the release runbook, the sandbox |
| `package.json` | JavaScript | the two tools that are npm packages: the backlog board and the skills installer |
| `pixi.toml`, `pixi.lock` | TOML | the one environment all of the above run in |

The languages are a consequence, not a choice: Rust is what the product is, Python is what
the checks are written in because they are text processing and process orchestration, and
the two JavaScript packages are tools that exist on npm and nowhere else. Pixi is what
makes them one command — `pixi run automation gates` runs a Rust linter, a Python checker,
a bun-installed binary and Vale in the same process tree, all pinned by one lock file.

That is the property worth keeping: **the environment is declared once and solved once**.
A contributor does not install Rust, then Python, then Vale; `scripts/bootstrap.sh` puts
the repository at its remote head, restores the environment and installs the git hooks.

## Why the examples are separate crates

`crates/clifacet` cannot use itself the way a caller does. An example inside it would be a
module with access to `pub(crate)` items, so it could keep working after a change that
breaks every external caller — the exact failure a reference application exists to catch.

`examples/simple` and `examples/backlog-lite` are therefore workspace members with their
own manifests, and they depend on `clifacet` by path and by version. `cargo test
--workspace` compiles them as a consumer would, and a change that removes or narrows a
public item fails there before it fails on crates.io. `examples/backlog-lite` is the
harder case on purpose: it wraps somebody else's CLI, so it exercises the API the way a
third-party author would rather than the way this repository finds convenient.

`examples/discovered` is the smallest of the three and answers one question — whether a
`build.rs` can register a folder of commands — so it stays small enough to read in one
sitting.

## Why the automation is Python and the task graph is doit

The checks are mostly "read these files, run these commands, and decide whether the repository is what it says it is." That is text and subprocess work, which Python does without a
build step, and it made the checks cheap to write and cheap to test.

What it did not have was a task graph. Each check was a Pixi task with a command string,
and the ordering lived in YAML: a gate that needed another gate said so in a `depends-on`,
and the only way to discover what existed was to read `pixi.toml`. The tasks are now
declared in `dodo.py`, which is a Python file with one function per task, and run through
`pixi run automation <task>`:

```text
scripts/<name>.py     the implementation: main(argv) -> int, runnable on its own
automation/lib/       the shared code: the bridge from doit to a script's main
dodo.py               the registry: names, actions, parameters and order
pixi.toml             the environment, and one alias per task for muscle memory
```

The division is deliberate and each part is load-bearing. Pixi owns the environment and
nothing else. doit owns the graph — what runs, in what order, with what parameters.
Python owns the implementation. A script that only works through doit would be a script
its own tests could not run, so every module keeps its `main` and its `__main__` guard.

## What is generated, and what that means

Several files are rendered from another file rather than written, and each one is
committed so that a change to the source is visible as a change to the result:

| Rendered file | From | Regenerate with |
| --- | --- | --- |
| `README.md` | `.readme/README.template.md` | `pixi run readme` |
| `CHANGELOG.md` | the conventional commits | `pixi run changelog` |
| `.vale.ini` and `.vale/styles/` | `vale.toml` | `pixi run prose-sync` |
| `examples/simple/tests/documented_snippets.rs` | the ```` ```rust ```` fences in the pages | `pixi run docs-snippets` |
| `scripts/restore.sh`, two sandbox workflows | `pixi-sandbox.toml` | `pixi-sandbox init` |

The committed copy is what CI checks, and the check fails rather than regenerating: a
generated file that is stale in a pull request means the author edited the wrong one, and
the failure is where they find out.

## Where the boundaries are, and why

- **The library does not know the repository exists.** No crate reads a file from the
  repository at run time; `examples/` are consumers, not extensions.
- **The automation does not import the library.** The checks read the sources as text and
  run cargo; none of them links against `clifacet`, so a broken build still lets the
  checks explain why.
- **The documentation is checked by the scripts, not by reading.** `scripts/check_docs.py`
  compiles the fences and resolves the paths, so a page cannot claim an API that was
  removed.
- **Planning is not documentation.** [`backlog/`](../../backlog/) holds decisions and
  tasks; the pages here describe the product. A decision that changes behavior is
  reflected here, and the reasoning stays in the decision.

The rest of the repository's rules — the checks, the hooks, the release — are in
[Development](../development/development.md).
