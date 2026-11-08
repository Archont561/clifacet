#!/usr/bin/env python3
"""Render and verify the prose check: `vale.toml` into `.vale.ini` and `.vale/styles/`.

Vale reads an INI file and a directory of YAML rules. Both are configuration, and both
would normally be hand-edited — which is how a style ends up with a rule nobody can
explain and a switch nobody remembers making. So neither is authored here. `vale.toml`
holds the decisions, each rule it turns off carries the reason, and this script renders
the two files Vale actually reads.

The rendered tree is committed, and `--check` fails when it has drifted, so a change to
`vale.toml` that nobody regenerated is caught in CI rather than discovered by a reviewer.
That is the same arrangement `scripts/restore.sh` and the documented snippets already use.

What this script can and cannot prove:

- It proves the rendering is deterministic and current, that every rule named in
  `vale.toml` exists in the pinned packages or in the style this repository owns, and
  that every rule switched off says why.
- It does not prove what Vale will report. Vale is not run here: the packages it needs
  are pinned and fetched by `pixi run prose-sync`, and the reports belong to
  `pixi run prose-check` in CI.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / "vale.toml"
INI = ROOT / ".vale.ini"
STYLES = ROOT / ".vale" / "styles"
STYLE = "Clifacet"

# The rule kinds this repository's own style may use, and the keys each one requires or
# accepts. Vale rejects an unknown key in a rule file, so a typo here would only surface
# in a lint run; naming them is what makes this script able to catch it.
CHECKS: dict[str, frozenset[str]] = {
    "existence": frozenset({"tokens", "raw", "ignorecase", "nonword", "scope", "exceptions"}),
    "substitution": frozenset({"swap", "ignorecase", "nonword", "scope", "exceptions"}),
}
COMMON_KEYS = frozenset({"extends", "message", "level", "link", "description", "action", "limit"})


def scalar(value: Any) -> str:
    """One YAML scalar, quoted only when it has to be.

    A hand-written mapping is not good enough here: `str(dict)` writes Python's repr, and
    `True` is not what YAML calls a boolean. Rules are rendered by this function so the
    file Vale reads is YAML rather than something that merely looks like it.
    """
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    text = str(value)
    # Quote when the value would otherwise be read as something other than a string, or
    # when it contains the characters a plain scalar cannot hold safely.
    if text and not re.search(r"[^A-Za-z0-9 _./$-]", text) and text.strip() == text:
        return text
    escaped = text.replace("\\", "\\\\").replace('"', '\\"')
    return f'"{escaped}"'


class ConfigError(Exception):
    """`vale.toml` says something the rendered tree could not honour."""


@dataclass(frozen=True)
class Rule:
    """One rule of the repository's own style."""

    name: str
    extends: str
    message: str
    level: str
    body: dict[str, Any] = field(default_factory=dict)

    def render(self) -> str:
        """The rule file Vale reads: the check, what it says, and its arguments."""
        lines = [
            f"extends: {self.extends}",
            f"message: {scalar(self.message)}",
            f"level: {self.level}",
        ]
        for key, value in self.body.items():
            if isinstance(value, dict):
                lines.append(f"{key}:")
                lines.extend(f"  {scalar(k)}: {scalar(v)}" for k, v in value.items())
            else:
                lines.append(f"{key}: {scalar(value)}")
        return "\n".join(lines) + "\n"


def load(path: Path = CONFIG) -> dict[str, Any]:
    """The configuration, validated far enough that rendering cannot fail halfway."""
    try:
        config = tomllib.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise ConfigError(f"{path} is missing; it is the source of the prose check") from error
    except tomllib.TOMLDecodeError as error:
        raise ConfigError(f"{path} is not valid TOML: {error}") from error

    for key in ("sources", "packages", "based_on", "vocab"):
        if key not in config:
            raise ConfigError(f"{path} does not set `{key}`")
    if not config["sources"]:
        raise ConfigError(f"{path} names no `sources`, so nothing would be checked")
    if not config["packages"]:
        raise ConfigError(
            f"{path} pins no `packages`; a style name installs whatever is newest, which is "
            "not a decision this repository can review"
        )
    for package in config["packages"]:
        if not package.startswith("https://"):
            raise ConfigError(
                f"{path} names the package `{package}`; give the release URL so the rules "
                "cannot change without a commit here"
            )

    rules = config.get("rules", {})
    for name, rule in rules.items():
        style, _, rule_name = name.partition(".")
        if style == STYLE:
            continue
        if style in (STYLE, "Vale") or not rule_name:
            continue
        # A rule this repository turns off or re-levels has to say why: the reason is the
        # only record of a decision that would otherwise be invisible in the INI.
        if str(rule.get("level", "")).upper() == "NO" and not rule.get("reason", "").strip():
            raise ConfigError(f"{path} switches `{name}` off without a `reason`")
    return config


def own_rules(config: dict[str, Any]) -> list[Rule]:
    """The rules of the repository's own style, in the order `vale.toml` lists them."""
    rules = []
    for name, spec in config.get("rules", {}).items():
        style, _, rule_name = name.partition(".")
        if style != STYLE or not rule_name:
            continue
        extends = spec.get("extends")
        if extends not in CHECKS:
            raise ConfigError(
                f"{name} extends `{extends}`; this repository's style allows "
                f"{', '.join(sorted(CHECKS))}"
            )
        unknown = set(spec) - {"extends", "message", "level"} - CHECKS[extends] - {"reason"}
        if unknown:
            raise ConfigError(f"{name} sets {', '.join(sorted(unknown))}, which {extends} has no key for")
        if not spec.get("message"):
            raise ConfigError(f"{name} has no `message`, so an alert would say nothing")
        body = {key: value for key, value in spec.items() if key in CHECKS[extends]}
        rules.append(Rule(rule_name, extends, spec["message"], spec.get("level", "warning"), body))
    return rules


def render_style(config: dict[str, Any]) -> dict[Path, str]:
    """Every file of the repository's own style, keyed by where it belongs."""
    files = {}
    for rule in own_rules(config):
        files[STYLES / STYLE / f"{rule.name}.yml"] = rule.render()
    accept = config.get("accept", [])
    if not accept:
        raise ConfigError("no `accept` entries: the project's own words would be spelling errors")
    files[STYLES / "config" / "vocabularies" / config["vocab"] / "accept.txt"] = (
        "\n".join(accept) + "\n"
    )
    return files


def render_ini(config: dict[str, Any]) -> str:
    """`.vale.ini`, with each decision and the reason behind it beside the rule it sets."""
    lines = [
        "# Generated by scripts/prose.py from vale.toml. Edit vale.toml, not this file.",
        "#",
        "# Vale checks how the guides read. Alint (`alint check`) checks what the repository",
        "# is: files that must exist, naming, manifests, workflows. The split is deliberate —",
        "# a file-existence rule in Vale would be a rule nobody can run, and a style",
        "# preference in .alint.yml would be a check that cannot see a sentence.",
        "",
        "StylesPath = .vale/styles",
        f"MinAlertLevel = {config['min_alert_level']}",
        "",
        "# Pinned releases, not names: `pixi run prose-sync` installs exactly these, so an",
        "# upstream update cannot change what CI rejects without a commit here.",
        "Packages = " + ", ".join(config["packages"]),
        "",
        f"Vocab = {config['vocab']}",
        "",
        "[*.md]",
        "BasedOnStyles = " + ", ".join(config["based_on"]),
    ]
    overrides = {
        name: spec
        for name, spec in config.get("rules", {}).items()
        if not name.startswith(f"{STYLE}.")
    }
    if overrides:
        lines.append("")
        lines.append("# The inherited rules this repository sets its own way, each because the guides")
        lines.append("# are written that way on purpose. The reasons are in vale.toml.")
        for name, spec in overrides.items():
            level = spec.get("level")
            if level:
                lines.append(f"{name} = {level}")
                reason = " ".join(spec.get("reason", "").split())
                if reason:
                    lines.append(f"#   {reason}")

    # Files that are not the project's prose: tool-shaped or generated, and already checked
    # by `backlog-check` and `changelog-check`. They stay in the INI so a whole-tree run
    # still reads them as Markdown rather than reporting them as unchecked.
    lines += ["", "[backlog/**/*.md]", "BasedOnStyles = Vale", "", "[CHANGELOG.md]", "BasedOnStyles = Vale", ""]
    return "\n".join(lines)


def rendered(config: dict[str, Any] | None = None) -> dict[Path, str]:
    """Everything the prose check generates, as one mapping of path to content."""
    config = load() if config is None else config
    files = {INI: render_ini(config)}
    files.update(render_style(config))
    return files


def differences(config: dict[str, Any] | None = None) -> list[str]:
    """How the committed tree differs from what `vale.toml` renders now."""
    problems = []
    for path, content in rendered(config).items():
        relative = path.relative_to(ROOT)
        try:
            present = path.read_text(encoding="utf-8")
        except FileNotFoundError:
            problems.append(f"{relative}: missing; run `pixi run prose-sync`")
            continue
        if present != content:
            problems.append(f"{relative}: out of date; run `pixi run prose-sync`")
    return problems


def write(config: dict[str, Any] | None = None) -> list[Path]:
    """Render the tree, returning every path written."""
    written = []
    for path, content in rendered(config).items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        written.append(path)
    return written


def sources(config: dict[str, Any] | None = None) -> list[str]:
    """The prose `prose-check` lints, as paths relative to the repository root."""
    config = load() if config is None else config
    files: list[str] = []
    for entry in config["sources"]:
        path = ROOT / entry
        if path.is_dir():
            files.extend(sorted(str(p.relative_to(ROOT)) for p in path.rglob("*.md")))
        elif path.is_file():
            files.append(entry)
        else:
            raise ConfigError(f"`sources` names {entry}, which is not a file or a directory")
    return files


def vale_command(config: dict[str, Any] | None = None) -> list[str]:
    """The `vale` arguments `prose-check` runs, as a list so a test can read them."""
    config = load() if config is None else config
    return ["vale", "--config", str(INI.relative_to(ROOT)), *sources(config)]


def sync(config: dict[str, Any] | None = None) -> int:
    """Install the pinned packages into the StylesPath, as `vale sync` does."""
    import subprocess

    config = load() if config is None else config
    styles = STYLES.relative_to(ROOT)
    styles.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        ["vale", "sync", f"--config={INI.relative_to(ROOT)}"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        print(f"prose: `vale sync` failed; {styles} is unchanged", file=sys.stderr)
        return result.returncode
    print(f"prose: synced {len(config['packages'])} package(s) into {styles}")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        choices=("check", "write", "sync", "list", "command"),
        nargs="?",
        default="check",
        help="check the committed tree, write it, sync the packages, or print what would run",
    )
    parser.add_argument("--format", choices=("text", "json"), default="text", help="`list` output")
    arguments = parser.parse_args(argv)

    try:
        config = load()
        if arguments.action == "write":
            for path in write(config):
                print(f"prose: wrote {path.relative_to(ROOT)}")
            return 0
        if arguments.action == "sync":
            return sync(config)
        if arguments.action == "list":
            print(json.dumps(sources(config)) if arguments.format == "json" else "\n".join(sources(config)))
            return 0
        if arguments.action == "command":
            print(" ".join(vale_command(config)))
            return 0
        problems = differences(config)
    except ConfigError as error:
        print(f"prose: {error}", file=sys.stderr)
        return 1

    if problems:
        for problem in problems:
            print(f"prose: {problem}", file=sys.stderr)
        return 1
    print(f"prose: {len(sources(config))} page(s), {len(rendered(config))} generated file(s), current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
