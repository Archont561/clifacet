#!/usr/bin/env python3
"""Validate project-owned Agent Skills; no remote installation or mutation."""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
REQUIRED = frozenset({
    "session", "regain-direction", "backlog", "tdd", "bdd", "audit", "refactor",
    "clean-code-principles", "grill-me", "writing-for-agents", "skill-creator",
})
NAME = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def check_skills(root: Path) -> list[str]:
    """Check required skills and simple frontmatter on every local skill directory."""
    problems = []
    directory = root / ".agents" / "skills"
    if not directory.is_dir() or directory.is_symlink():
        return [f"missing or symlinked skills directory: {directory}"]
    present = {p.name for p in directory.iterdir() if p.is_dir() or p.is_symlink()}
    for name in sorted(REQUIRED - present):
        problems.append(f"missing required skill: {name}")
    for name in sorted(present):
        folder = directory / name
        path = folder / "SKILL.md"
        if not NAME.fullmatch(name):
            problems.append(f"invalid skill directory name: {name}")
        if folder.is_symlink() or path.is_symlink() or not path.is_file():
            problems.append(f"missing or symlinked SKILL.md: {name}")
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            problems.append(f"cannot read {path}: {error}")
            continue
        parts = text.split("---\n", 2)
        if len(parts) != 3 or parts[0] != "" or not parts[2].strip():
            problems.append(f"invalid frontmatter or empty body: {name}")
            continue
        fields = {}
        for line in parts[1].splitlines():
            key, separator, value = line.partition(":")
            if separator and key in ("name", "description"):
                if key in fields:
                    problems.append(f"duplicate {key}: {name}")
                fields[key] = value.strip().strip('"\'')
        if fields.get("name") != name:
            problems.append(f"frontmatter name does not match directory: {name}")
        if not fields.get("description"):
            problems.append(f"missing description: {name}")
    return problems


def main() -> int:
    problems = check_skills(ROOT)
    if problems:
        for problem in problems:
            print(f"skills-check: {problem}", file=sys.stderr)
        return 1
    print(f"skills-check: {len(REQUIRED)} required local skills validated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
