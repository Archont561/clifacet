"""The repository's automation, as one task graph.

    pixi run automation list
    pixi run automation docs
    pixi run automation gates

Every task here is a name, an action and the tasks it needs; the work stays in `scripts/`,
which each module still runs on its own (`python scripts/prose.py`). The split is deliberate
— a script reachable only through doit is a script its own tests cannot run — so this file is
a registry, not an implementation.

Two properties are worth stating because they are not obvious from the code:

- **Nothing is cached.** Doit can skip a task whose `file_dep` predate its `targets`, and no
  check here has one: a check that skipped itself because no input changed is a check that
  reports success without looking. The two tasks that write files say so in their names
  (`docs_write`, `changelog`) and are the only way to change them.
- **The order is the graph.** `gates` lists its dependencies cheapest-to-fail first, so the
  most common mistake reports before the most expensive work runs.
"""

from __future__ import annotations

import subprocess
import sys
from collections.abc import Callable

from automation.lib.tasks import run

# What a task action returns; doit reads an action's exception, not its value, so the
# helpers below either return `None` or raise.
Action = Callable[[], None]


def shell(command: list[str]) -> Action:
    """An action that runs a command and fails the task on a non-zero exit.

    `subprocess.run` without `check` would report a failed check as a passed task, which is
    the one mistake a task runner must not make, so the exit code is turned into an exception
    here rather than trusted to the caller.
    """

    def action() -> None:
        finished = subprocess.run(command, check=False)  # noqa: S603 — fixed argument lists
        if finished.returncode:
            message = f"{' '.join(command)} exited {finished.returncode}"
            raise RuntimeError(message)

    return action


def cargo(*arguments: str) -> Action:
    """An action that runs `cargo` from the environment Pixi activated."""
    return shell(["cargo", *arguments])


def script(name: str, **arguments: str) -> Action:
    """An action that runs `scripts/<name>.py`'s `main`.

    Each keyword is one long flag, with underscores spelled as hyphens: `write=""` is
    `--write`, and a keyword's value is its argument, so `coverage="path"` is
    `--coverage path`. A script whose `main` takes no `argv` is called bare.
    """

    def action() -> None:
        argv = [
            argument
            for flag, value in arguments.items()
            for argument in (
                (f"--{flag.replace('_', '-')}",)
                if value == ""
                else (f"--{flag.replace('_', '-')}", value)
            )
        ]
        run(name, argv=argv)

    return action


def positional(name: str, *arguments: str) -> Action:
    """An action that runs `scripts/<name>.py`'s `main` with positional arguments.

    `changelog` takes `regen`, `check` or `notes`, and `prose` takes `check`, `write`,
    `lint` or `sync`: subcommands rather than flags. Passing those through `script()`,
    which writes long flags, produces a task that runs and then fails on its own
    argument parser — a mistake worth a separate helper rather than a comment.
    """

    def action() -> None:
        run(name, argv=list(arguments))

    return action


def task_fmt() -> dict:
    """Rewrite the Rust sources in place."""
    return {"actions": [cargo("fmt")]}


def task_fmt_check() -> dict:
    """Fail when a Rust file is not formatted."""
    return {"actions": [cargo("fmt", "--all", "--", "--check")]}


def task_lint() -> dict:
    """Clippy over the workspace, its examples and its tests, warnings denied."""
    return {
        "actions": [
            cargo(
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--locked",
                "--",
                "-D",
                "warnings",
            )
        ]
    }


def task_test_rust() -> dict:
    """The library and integration targets, through nextest."""
    return {"actions": [cargo("nextest", "run", "--workspace", "--all-features", "--locked")]}


def task_test_doc() -> dict:
    """The doctests nextest does not run."""
    return {
        "actions": [cargo("test", "--workspace", "--all-features", "--doc", "--locked")]
    }


def task_test_scripts() -> dict:
    """The repository's own Python suite, which covers the scripts this file drives."""
    return {"actions": [shell([sys.executable, "-m", "pytest", "scripts/tests"])]}


def task_test() -> dict:
    """Every suite: Rust, doctests and the repository scripts."""
    return {"actions": [], "task_dep": ["test_rust", "test_doc", "test_scripts"]}


def task_alint() -> dict:
    """Repository policy: the files that must exist, and the manifests that must agree."""
    return {
        "actions": [
            shell(
                [
                    "alint",
                    "check",
                    "--baseline",
                    ".alint-baseline.json",
                    "--strict-baseline",
                    "--fail-on-warning",
                ]
            )
        ]
    }


def task_markdownlint() -> dict:
    """Lint the shape of every page: headings, lists, fences, blank lines.

    Deliberately separate from `prose`: Vale owns the words, this owns the structure, and
    a failure should say which of the two a page got wrong. `markdownlint-cli2` is an npm
    tool, so it needs the network the first time and is therefore not in `gates`.
    """
    return {
        "actions": [
            shell(
                [
                    "bun",
                    "x",
                    "--bun",
                    "--no-install",
                    "markdownlint-cli2@0.18.1",
                ]
            )
        ]
    }


def task_prose() -> dict:
    """Vale over the guides: the committed rendering is current, and the pages pass."""
    return {"actions": [positional("prose", "check"), positional("prose", "lint")]}


def task_docs() -> dict:
    """The documentation gate: links, anchors, compiled snippets and real API names."""
    return {"actions": [script("check_docs")], "task_dep": ["prose"]}


def task_docs_write() -> dict:
    """Regenerate the compiled-snippet test file after editing a page. Writes; never gates."""
    return {"actions": [script("check_docs", write="")]}


def task_readme() -> dict:
    """Render README.md from the template with readme-md-generator. Writes; never gates."""
    return {"actions": [script("readme")]}


def task_readme_check() -> dict:
    """README.md is what the template renders, and the generator needs the network.

    Not in `gates` for that reason: a restored sandbox has no npm cache, so a gate that
    re-renders would fail for the network rather than for the README. CI runs it, where
    the fetch is allowed.
    """
    return {"actions": [script("readme", check="")]}


def task_changelog_check() -> dict:
    """The committed changelog matches the commits it was rendered from."""
    return {"actions": [positional("changelog", "check")]}


def task_changelog() -> dict:
    """Render CHANGELOG.md from the conventional commits. Writes; never gates."""
    return {"actions": [positional("changelog", "regen")]}


def task_msrv() -> dict:
    """Check every feature boundary with the compiler the manifest declares as its floor.

    The toolchain is a parameter because CI installs one and a local session may not have
    it: `--toolchain ""` reuses the pinned compiler as a plumbing check, which is what the
    task did before it had a name here.
    """
    return {
        "actions": [
            lambda toolchain: run("prove_msrv", argv=["--toolchain", toolchain])
        ],
        "params": [
            {
                "name": "toolchain",
                "long": "toolchain",
                "short": "t",
                "default": "1.88",
                "type": str,
                "help": "Rust toolchain to check; empty reuses the pinned compiler",
            }
        ],
        "verbosity": 2,
    }


def task_check_features() -> dict:
    """Compile every supported feature combination, alone and together."""
    return {"actions": [script("check_features")]}


def task_check_advisory() -> dict:
    """The unmaintained-crate exception still applies and has not expired."""
    return {"actions": [script("check_advisory_exception")]}


def task_check_packages() -> dict:
    """What `cargo publish` would upload holds the licences and not the scaffolding."""
    return {"actions": [script("check_packages")]}


def task_actionlint() -> dict:
    """Lint the workflow files, with shellcheck reading their `run:` blocks."""
    return {"actions": [shell(["actionlint", ".github/workflows/ci.yml",
                               ".github/workflows/release.yml",
                               ".github/workflows/relock.yml",
                               ".github/workflows/publish-sandbox.yml"])]}


def task_sandbox_check() -> dict:
    """The generated restore launcher and workflows match the authored sandbox policy."""
    return {
        "actions": [
            shell(
                [
                    "pixi-sandbox",
                    "init",
                    "--config",
                    "pixi-sandbox.toml",
                    "--script-path",
                    "scripts/restore.sh",
                    "--check",
                ]
            )
        ]
    }


def task_backlog_check() -> dict:
    """The board is a valid graph, and every card's status agrees with its criteria."""
    return {
        "actions": [
            shell(["bun", "x", "--bun", "--no-install", "backlog", "doctor"]),
            script("check_backlog_status"),
        ]
    }


def task_skills_check() -> dict:
    """Every skill the agents rely on is present and has valid frontmatter."""
    return {"actions": [script("check_skills")]}


def task_audit() -> dict:
    """Advisories from the RustSec database. Needs the network, so it is not in `gates`."""
    return {"actions": [cargo("audit", "--deny", "warnings")]}


def task_deny() -> dict:
    """Licences, bans, sources and advisories. Needs the network, so not in `gates`."""
    return {"actions": [cargo("deny", "check")]}


def task_coverage() -> dict:
    """Line coverage from nextest, checked against the floor `codecov.yml` declares."""
    return {
        "actions": [
            cargo(
                "llvm-cov",
                "nextest",
                "--workspace",
                "--all-features",
                "--locked",
                "--lcov",
                "--output-path",
                "coverage/lcov.info",
            ),
            script("check_coverage", coverage="coverage/lcov.info"),
        ]
    }


def task_bootstrap() -> dict:
    """Reach the remote head, restore the environment, install the hooks, prove it."""
    return {"actions": [shell(["bash", "scripts/bootstrap.sh"])]}


def task_gates() -> dict:
    """Everything a change must pass before it is proposed, cheapest failure first.

    Offline by construction, apart from the dependency gates: `audit` and `deny` fetch the
    RustSec database and belong to `gates_online`, which CI runs. This is the task a
    contributor runs after a restore.
    """
    return {
        "actions": [],
        "task_dep": [
            "fmt_check",
            "lint",
            "alint",
            "docs",
            "changelog_check",
            "skills_check",
            "check_features",
            "actionlint",
            "check_advisory",
            "check_packages",
            "sandbox_check",
            "test",
        ],
    }


def task_gates_online() -> dict:
    """`gates` plus the two checks that need the RustSec database."""
    return {"actions": [], "task_dep": ["gates", "audit", "deny"]}
