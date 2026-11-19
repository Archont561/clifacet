#!/usr/bin/env python3
"""Render `README.md` from `templates/README.md.j2`.

The facts come from this repository: the template's variables are derived from
`Cargo.toml` (`version`, `repository`, `author`, `license`).

- **The facts come from this repository.** The values are derived here from `Cargo.toml`.
- **The render is unattended and reproducible.**
- `--check` renders and compares instead of writing.

Run it with `pixi run readme` and check it with `pixi run readme-check`. Both go through
the Tasknest CLI as `tools run docs.readme`; neither is part of the gate, because a render
that rewrites a file is a person's action rather than a check's.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from collections.abc import Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))
TEMPLATE = ROOT / "templates" / "README.md.j2"
README = ROOT / "README.md"
PACKAGE = ROOT / "package.json"
CARGO = ROOT / "Cargo.toml"


def workspace_version() -> str:
    """The version the crate actually is, read where `bump_version.py` writes it."""
    manifest = tomllib.loads(CARGO.read_text(encoding="utf-8"))
    return str(manifest["workspace"]["package"]["version"])


def repository_url() -> str:
    """The canonical repository URL from the Cargo manifest, not from the git remote."""
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
    """The answers the renderer substitutes into the template."""
    return {
        "name": "clifacet",
        "version": workspace_version(),
        "description": "Commands as values: validated input in, data or a failure out.",
        "author": author(),
        "license": "MIT OR Apache-2.0",
        "repository": {"type": "git", "url": repository_url()},
    }


def render() -> str:
    """Read the template and substitute repository facts."""
    if not TEMPLATE.is_file():
        shown = TEMPLATE.relative_to(ROOT) if TEMPLATE.is_relative_to(ROOT) else TEMPLATE
        raise SystemExit(f"readme: the template is missing: {shown}")
    content = TEMPLATE.read_text(encoding="utf-8")
    ctx = context()
    values = {
        "version": ctx["version"],
        "projectVersion": ctx["version"],
        "author": ctx["author"],
        "authorName": ctx["author"],
        "repository": ctx["repository"]["url"],
        "repositoryUrl": ctx["repository"]["url"],
        "name": ctx["name"],
        "projectName": ctx["name"],
        "description": ctx["description"],
        "projectDescription": ctx["description"],
    }
    # Substitute Jinja2-style variables: {{ variable }}
    def replace_var(match: re.Match[str]) -> str:
        var_name = match.group(1).strip()
        if var_name in values:
            return values[var_name]
        return match.group(0)

    rendered = re.sub(r"\{\{\s*([a-zA-Z0-9_]+)\s*\}\}", replace_var, content)
    # Also substitute legacy <% tags if any are present
    for key, val in values.items():
        rendered = rendered.replace(f"<%= {key} %>", val)
    return rendered


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
                "readme: README.md is not what templates/README.md.j2 renders.\n"
                "        Edit the template, then run `pixi run readme`.",
                file=sys.stderr,
            )
            return 1
        print(f"readme: README.md matches the template ({len(rendered.splitlines())} lines)")
        return 0

    README.write_text(rendered, encoding="utf-8")
    print(
        f"readme: wrote README.md from templates/README.md.j2 "
        f"({len(rendered.splitlines())} lines)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
