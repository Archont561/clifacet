#!/usr/bin/env python3
"""Render `README.md` from `.readme/README.template.md` with readme-md-generator.

The generator is an npm package, and it was written for npm packages: it reads the
project's facts from the nearest `package.json`, asks the rest as questions, and writes
`README.md` into the working directory. None of that is true here — this repository's
`package.json` describes the development tooling rather than the crate, there is no
answer to give a terminal prompt, and the README already exists. Three things follow,
and this script is all three:

- **The facts come from this repository.** The generator's answers are derived here and
  written to a private `package.json` in a scratch directory, which is what the renderer
  reads. The committed `package.json` is untouched: its name and version belong to the
  tooling, and a README that called this crate `clifacet-dev` would be wrong in the first
  line.
- **The render is unattended.** `-y` takes the default answer to every question, so the
  template can only use values that were derived above rather than asked for.
- **The overwrite prompt never appears.** The generator asks before replacing an existing
  `README.md` and has no flag to skip it, so it renders into an empty directory and the
  result is copied into place. `--check` renders and compares instead of writing.

Run it with `pixi run readme` and check it with `pixi run readme-check`. Both go through
the automation CLI as `automation readme`; neither is part of the gate, because a render
that rewrites a file is a person's action rather than a check's.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TEMPLATE = ROOT / ".readme" / "README.template.md"
README = ROOT / "README.md"
PACKAGE = ROOT / "package.json"
CARGO = ROOT / "Cargo.toml"

# Pinned, like every other tool this repository fetches: a name would install whatever is
# newest, so the README could change without a commit that says why.
GENERATOR = "readme-md-generator@1.0.0"


def bun() -> str:
    """The `bun` on PATH, which Pixi provides, or a message instead of a traceback."""
    found = shutil.which("bun")
    if found is None:
        raise SystemExit(
            "readme: bun is not on PATH. Run this through Pixi "
            "(`pixi run readme`) so the environment provides it."
        )
    return found


def workspace_version() -> str:
    """The version the crate actually is, read where `bump_version.py` writes it."""
    manifest = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    return str(manifest["workspace"]["package"]["version"])


def repository_url() -> str:
    """The canonical repository URL from the Cargo manifest, not from the git remote.

    A checkout's remote can be a fork or an SSH alias; the URL published with the crate is
    the one a reader should follow.
    """
    manifest = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    return str(manifest["workspace"]["package"]["repository"])


def author() -> str:
    """The first author, with the `Name <address>` form reduced to the name."""
    manifest = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    authors = manifest["workspace"]["package"].get("authors") or []
    if not authors:
        return ""
    return str(authors[0]).split("<", 1)[0].strip()


def context() -> dict[str, str]:
    """The answers the generator reads, shaped as the `package.json` fields it looks for.

    Only the fields this template uses are set. A field left out becomes a prompt, and a
    prompt with `-y` becomes a default nobody chose, so the list stays explicit.
    """
    package = json.loads(PACKAGE.read_text(encoding="utf-8"))
    return {
        "name": "clifacet",
        "version": workspace_version(),
        "description": "Commands as values: validated input in, data or a failure out.",
        "author": author(),
        "license": "MIT OR Apache-2.0",
        "repository": {"type": "git", "url": repository_url()},
        # Kept from the committed manifest so `bun install` still resolves the same
        # tooling if the scratch directory is inspected while debugging.
        "devDependencies": package.get("devDependencies", {}),
        "private": True,
    }


def render() -> str:
    """Run the generator in an empty directory and return what it wrote."""
    if not TEMPLATE.is_file():
        shown = TEMPLATE.relative_to(ROOT) if TEMPLATE.is_relative_to(ROOT) else TEMPLATE
        raise SystemExit(f"readme: the template is missing: {shown}")
    with tempfile.TemporaryDirectory(prefix="clifacet-readme-") as directory:
        work = Path(directory)
        (work / "package.json").write_text(json.dumps(context(), indent=2), encoding="utf-8")
        # The generator resolves the template path relative to where it runs, and it
        # requires that path to exist, so the template is copied in beside the manifest.
        shutil.copyfile(TEMPLATE, work / "template.md")
        finished = subprocess.run(  # noqa: S603 — a fixed argument list, no shell
            [bun(), "x", GENERATOR, "-y", "-p", "template.md"],
            cwd=work,
            capture_output=True,
            text=True,
            check=False,
        )
        rendered = work / "README.md"
        if finished.returncode or not rendered.is_file():
            raise SystemExit(
                "readme: the generator failed\n"
                f"{finished.stdout}\n{finished.stderr}".strip()
            )
        return rendered.read_text(encoding="utf-8")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail if README.md differs from what the template renders, without writing",
    )
    arguments = parser.parse_args(argv)

    rendered = render()

    if arguments.check:
        current = README.read_text(encoding="utf-8") if README.is_file() else ""
        if current != rendered:
            print(
                "readme: README.md is not what .readme/README.template.md renders.\n"
                "        Edit the template, then run `pixi run readme`.",
                file=sys.stderr,
            )
            return 1
        print(f"readme: README.md matches the template ({len(rendered.splitlines())} lines)")
        return 0

    README.write_text(rendered, encoding="utf-8")
    print(f"readme: wrote README.md from .readme/README.template.md "
          f"({len(rendered.splitlines())} lines)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
