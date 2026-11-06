#!/usr/bin/env python3
"""Render `CHANGELOG.md` from the conventional commits, with convco.

The changelog used to be hand-written, and drifted from the commits it described: entries
merged between a release's promotion and its tag shipped inside the tarball while sitting
under `Unreleased`. This script removes the second author. `convco changelog` reads the
same commit messages the `commit-msg` hook already validates, `.versionrc` decides which
types earn a section, and the rendering below turns convco's output into Keep a Changelog:

- **headings.** convco writes `## v0.1.0 (2026-11-01)`, and writes a *patch* version one
  level deeper than a minor one (`###`), which is a detail of its default template, not
  something a reader of the changelog should have to notice. Both become
  `## [0.1.0] - 2026-11-01`, and the sections under them become `###`.
- **links.** The compare URL convco puts inside the heading becomes a reference at the
  bottom of the file, which is where Keep a Changelog keeps them.
- **whitespace.** Runs of blank lines collapse to one, every heading gets exactly one
  blank line above it, and the file ends with a single newline — so re-rendering is
  byte-stable and `check` means something.

`notes` is the same renderer applied to the commits that have not been tagged yet: that is
what `release.py` attaches to the GitHub Release, and it refuses an empty set instead of
publishing a release with nothing to say about itself.

Subcommands: `regen` (write the file), `check` (fail when the committed copy is stale),
`print`, `notes VERSION`, and `commit` (regenerate, then commit and push — the step
`release.yml` runs after tagging, since a version section cannot exist before its tag).
"""

from __future__ import annotations

import argparse
import datetime
import os
import re
import shutil
import subprocess
import sys
from collections.abc import Callable, Sequence
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = "CHANGELOG.md"
_OWNER = "Archont561"
_REPOSITORY = "clifacet"
COMMIT_MESSAGE = "chore(release): regenerate the changelog"

Output = Callable[[Sequence[str]], "tuple[int, str]"]

# `## [Unreleased](compare) (date)`, `## v0.1.0 (date)`, `## [0.2.0] - 2026-12-01`, and the
# `###` variants convco uses for patch versions and for its own re-rendering.
VERSION_HEADING = re.compile(
    r"""^\#+\s+
        (?:
            \[(?P<bracket>[^\]]+)\](?:\((?P<url>[^)]*)\))?
          | (?P<plain>v?\d+\.\d+\.\d+(?:-[0-9A-Za-z.]+)?)
          | (?P<word>Unreleased)
        )
        (?:\s+"[^"]*")?
        (?:\s+-\s+(?P<dash_date>\d{4}-\d{2}-\d{2}))?
        (?:\s+\((?P<paren_date>\d{4}-\d{2}-\d{2})\))?
        \s*$""",
    re.VERBOSE,
)

HEADING = re.compile(r"^#+\s+")
UNRELEASED = "Unreleased"


class ChangelogError(RuntimeError):
    """The changelog cannot be rendered or committed; the message says what to do."""


class UnreleasedError(ChangelogError):
    """The commits since the last tag say nothing, so there is nothing to release."""


def _version_heading(line: str) -> tuple[str, str | None, str | None] | None:
    """Read a version heading as `(name, compare url, date)`, or `None` if it is not one."""
    match = VERSION_HEADING.match(line)
    if match is None:
        return None
    name = match.group("bracket") or match.group("plain") or match.group("word")
    name = re.sub(r"^v", "", name.strip())
    date = match.group("dash_date") or match.group("paren_date")
    return name, match.group("url"), date


def render(raw: str, *, include_links: bool = True) -> str:
    """Normalise convco's output into the changelog this repository publishes."""
    lines: list[str] = []
    links: list[tuple[str, str]] = []
    seen_version = False

    for line in raw.splitlines():
        heading = _version_heading(line)
        if heading is not None:
            name, url, date = heading
            seen_version = True
            # `Unreleased` is not a date, and its compare URL cannot name a commit: either
            # would rewrite the file on every commit that touches nothing a user reads.
            if name == UNRELEASED:
                lines.append(f"## [{UNRELEASED}]")
                if url:
                    links.append((name, re.sub(r"\.\.\.[^/]+$", "...HEAD", url)))
                continue
            lines.append(f"## [{name}] - {date}" if date else f"## [{name}]")
            if url:
                links.append((name, url))
            continue
        if seen_version and HEADING.match(line):
            # Everything inside a version section is one level under it, whoever wrote the
            # original heading at whatever depth.
            lines.append(f"### {HEADING.sub('', line).strip()}")
            continue
        lines.append(line.rstrip())

    text: list[str] = []
    for line in lines:
        if not line.strip():
            if text and text[-1] != "":
                text.append("")
            continue
        if line.startswith("#") and text and text[-1] != "":
            text.append("")
        text.append(line)
    while text and text[-1] == "":
        text.pop()

    if include_links and links:
        text.append("")
        text.extend(f"[{name}]: {url}" for name, url in links)
    return "\n".join(text) + "\n"


def unreleased_section(raw: str) -> list[str]:
    """The lines of the first version section in convco's output, body only."""
    body: list[str] = []
    inside = False
    for line in raw.splitlines():
        if _version_heading(line) is not None:
            if inside:
                break
            inside = True
            continue
        if inside:
            body.append(line)
    while body and not body[0].strip():
        body.pop(0)
    while body and not body[-1].strip():
        body.pop()
    return body


def notes(raw: str, version: str, date: str) -> str:
    """The release notes for `version`, from the commits that follow the last tag.

    `raw` is convco's output with `--unreleased <version>`, so the first section already
    holds those commits; it is re-headed here because convco cannot know the date the
    release will carry, and a compare link for a tag that does not exist yet would 404.
    """
    body = unreleased_section(raw)
    if not body or not any(line.strip() for line in body if not HEADING.match(line)):
        raise UnreleasedError(
            f"nothing to release: no conventional commit follows the last tag, so {version} "
            "would ship without a changelog entry. Commit something describable first."
        )
    return render("\n".join([f"## [{version}] - {date}", *body]), include_links=False)


def _run(argv: Sequence[str], root: Path) -> tuple[int, str]:
    try:
        result = subprocess.run(
            list(argv), cwd=root, capture_output=True, text=True, check=False
        )
    except OSError as error:
        raise ChangelogError(f"could not run {argv[0]}: {error}") from error
    return result.returncode, result.stdout + result.stderr


def _ensure_history(root: Path) -> None:
    """convco reads tags and commits, so a shallow clone cannot render anything truthful."""
    code, output = _run(["git", "rev-parse", "--is-shallow-repository"], root)
    if code == 0 and output.strip() == "true":
        raise ChangelogError(
            "this checkout is shallow, so the commit history convco needs is not here. "
            "Fetch it (`git fetch --unshallow`), or check out with `fetch-depth: 0`."
        )


def convco(root: Path) -> Output:
    """The real convco, configured for this checkout."""

    def run(argv: Sequence[str]) -> tuple[int, str]:
        _ensure_history(root)
        executable = shutil.which("convco")
        if executable is None:
            raise ChangelogError(
                "convco is not on PATH; run this through Pixi (`pixi run --frozen changelog`)"
            )
        return _run([executable, *argv], root)

    return run


def generate(runner: Output, root: Path) -> str:
    """Render the whole changelog from the repository's history."""
    code, output = runner(["changelog"])
    if code != 0:
        raise ChangelogError(f"convco changelog failed (exit {code}):\n{output.strip()}")
    rendered = render(output)
    if rendered.strip() == render("").strip():
        raise ChangelogError(
            "convco produced no versions; the history holds no release tag or no "
            "conventional commits to describe"
        )
    return rendered


def notes_for(
    runner: Output, version: str, date: str | None = None, root: Path = ROOT
) -> str:
    """Release notes for a version that has not been tagged yet."""
    code, output = runner(["changelog", "--unreleased", version])
    if code != 0:
        raise ChangelogError(f"convco changelog failed (exit {code}):\n{output.strip()}")
    return notes(output, version, date or datetime.datetime.now(datetime.UTC).date().isoformat())


def _push_url(branch: str) -> str:
    """Where to push, with the workflow token when there is one.

    `release.yml` checks out without persisting credentials, so the token arrives in the
    environment instead; it is never printed, and any failure scrubs it before reporting.
    """
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token:
        return f"https://x-access-token:{token}@github.com/{{owner}}/{{repo}}.git"
    return "origin"


def commit(
    root: Path,
    branch: str,
    message: str = COMMIT_MESSAGE,
    run: Output | None = None,
    push: Output | None = None,
    identity: tuple[str, str] | None = None,
) -> str:
    """Commit a regenerated changelog and push it to `branch`; report what happened.

    `identity` is a `(name, email)` to commit as, for a checkout with no git identity of
    its own — a release runner. Without one, git's configured identity is used, so a
    maintainer running this locally commits as themselves.
    """
    execute = run or (lambda argv: _run(argv, root))
    send = push or (lambda argv: _run(argv, root))
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN") or ""

    code, output = execute(["git", "status", "--porcelain", "--", CHANGELOG])
    if code != 0:
        raise ChangelogError(f"git status failed: {output.strip()}")
    if not output.strip():
        return "the changelog already matches the tag; nothing to commit"

    for argv, what in (
        (["git", "add", "--", CHANGELOG], "stage the changelog"),
        ([*_git_as(identity), "commit", "-m", message, "--", CHANGELOG], "commit the changelog"),
    ):
        code, output = execute(argv)
        if code != 0:
            raise ChangelogError(f"could not {what}: {output.strip()}")

    remote = _push_url(branch).format(owner=_OWNER, repo=_REPOSITORY)
    code, output = send(["git", "push", remote, f"HEAD:refs/heads/{branch}"])
    if code != 0:
        redacted = output.replace(token, "***") if token else output
        raise ChangelogError(
            f"could not push the regenerated changelog to {branch}: {redacted.strip()}. "
            "Run `pixi run changelog` locally and commit the result."
        )
    return f"committed and pushed the regenerated changelog to {branch}"


def _git_as(identity: tuple[str, str] | None) -> list[str]:
    """`git`, with the identity to commit as given on the command line when there is one."""
    if identity is None:
        return ["git"]
    name, email = identity
    return ["git", "-c", f"user.name={name}", "-c", f"user.email={email}"]


def main(
    argv: Sequence[str] | None = None,
    root: Path = ROOT,
    runner: Output | None = None,
    git: Output | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action",
        nargs="?",
        default="regen",
        choices=["regen", "check", "print", "notes", "commit"],
        help="what to do with the rendered changelog",
    )
    parser.add_argument("version", nargs="?", help="the version to write notes for")
    parser.add_argument("--date", help="the release date for notes; defaults to today")
    parser.add_argument("--branch", default="main", help="the branch `commit` pushes to")
    arguments = parser.parse_args(argv)

    execute = runner or convco(root)
    changelog = Path(root) / CHANGELOG

    try:
        if arguments.action == "notes":
            if not arguments.version:
                parser.error("notes needs a version, for example `notes 0.2.0`")
            sys.stdout.write(notes_for(execute, arguments.version, arguments.date, root))
            return 0

        rendered = generate(execute, Path(root))
        if arguments.action == "print":
            sys.stdout.write(rendered)
            return 0
        if arguments.action == "check":
            current = changelog.read_text(encoding="utf-8") if changelog.is_file() else ""
            if current != rendered:
                print(
                    f"changelog: {CHANGELOG} does not match the rendered history. "
                    "Run `pixi run changelog` and commit the result.",
                    file=sys.stderr,
                )
                return 1
            print(f"changelog: {CHANGELOG} matches the rendered history")
            return 0
        changelog.write_text(rendered, encoding="utf-8")
        if arguments.action == "commit":
            print(f"changelog: {commit(Path(root), arguments.branch, run=git)}")
        else:
            print(f"changelog: wrote {CHANGELOG}")
        return 0
    except ChangelogError as error:
        print(f"changelog: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
