#!/usr/bin/env python3
"""Everything `release.yml` has to decide, decided here instead of in YAML.

The workflow is the part that cannot be tested: it runs once, on a maintainer's
dispatch, and the questions worth answering are answered then — at most once, and
sometimes only by a real publish. So the job keeps the plumbing (`environment`,
`id-token`, the action pins) and every decision moves here:

- **preflight** — the version the manifests record, the crates inherit it, and the
  commits since the last tag say something a release can be described by. All three are
  proved before anything is packaged, because a publish cannot be undone and the notes
  step used to run last.
- **changelog** — after the tag exists, the committed `CHANGELOG.md` is regenerated from
  the commits and pushed. It cannot be part of the release commit: a version section
  names a tag, and the tag is created by the publish that precedes it.
- **package** — one crate, `--locked`, so the tarball matches the committed lockfile.
- **report** — the dry run's summary: for each crate, whether this version is already on
  the index and what a publish run would therefore do.
- **publish** — macro first, then main, skipping any version already indexed. That skip
  is the recovery story: a run that died between the two publishes is finished by
  re-running the same command.
- **tag** — the GitHub Release, from the notes file that was verified, and left alone if
  it already exists.

The two effects are injected: `fetch` answers the sparse-index question as
`(status, body)`, and `run` executes cargo, git and gh. The rules over their results are
pure functions, which is what `scripts/tests/test_release.py` exercises.
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

# This file is run both as `python scripts/release.py` (Pixi, CI) and imported as
# `scripts.release` (the pytest suite). Only the second has the repository root on
# `sys.path`, so put it there before reaching for the sibling modules.
ROOT = Path(__file__).resolve().parent.parent
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.bump_version import crate_pins, inconsistencies, versions
from scripts.changelog import (
    CHANGELOG,
    ChangelogError,
    commit as commit_changelog,
    convco,
    generate,
    notes_for,
)
from scripts.github_actions import append_summary, set_error, set_output

INDEX = "https://index.crates.io"
# Sparse-index path for both crate names: two characters, then two more.
INDEX_PREFIX = "cl/if"

MACROS = "clifacet-macros"
MAIN = "clifacet"
# Publish order, not preference: the main crate's `clifacet-macros = "<version>"`
# requirement cannot resolve until that exact version is on the index.
ORDER = (MACROS, MAIN)

MAIN_REF = "refs/heads/main"
VERSION_ENTRY = re.compile(r'"vers"\s*:\s*"([^"]+)"')

Fetch = Callable[[str], "tuple[int, str]"]
Runner = Callable[[Sequence[str]], int]
Record = Callable[[str], None]


class ReleaseError(RuntimeError):
    """A release cannot proceed; the message says what to do about it."""


@dataclass(frozen=True)
class Poll:
    """How long to wait for a published version to reach the index."""

    attempts: int = 30
    delay: float = 10.0


@dataclass(frozen=True)
class Probe:
    """What the index says about one crate at one version."""

    crate: str
    version: str
    state: str  # "indexed", "absent" or "unreadable"
    detail: str = ""


@dataclass(frozen=True)
class Preflight:
    """What the later steps need to know, once the release is allowed to proceed."""

    version: str
    notes: str
    notes_path: Path


# Who the changelog commit is by. The runner has no git identity, and the 0.3.0 release's
# changelog step failed on `Author identity unknown` after the crates and the tag had
# shipped. This is the identity GitHub shows for commits made with the workflow token.
RELEASE_IDENTITY = (
    "github-actions[bot]",
    "41898282+github-actions[bot]@users.noreply.github.com",
)


def index_url(crate: str, prefix: str = INDEX_PREFIX) -> str:
    return f"{INDEX}/{prefix}/{crate}"


def versions_in(body: str) -> set[str]:
    """Every version the index entry lists."""
    return set(VERSION_ENTRY.findall(body))


def probe(fetch: Fetch, crate: str, version: str) -> Probe:
    """Whether `version` of `crate` is on the index.

    Three states, because two would conflate "not published yet" with "the index did not
    answer". The first means publish; the second means stop rather than publish blind.
    """
    status, body = fetch(index_url(crate))
    if status == 200:
        state = "indexed" if version in versions_in(body) else "absent"
        return Probe(crate, version, state)
    if status == 404:
        return Probe(crate, version, "absent", "not on the index at all")
    detail = f"HTTP {status}" if status else body.strip() or "the index could not be reached"
    return Probe(crate, version, "unreadable", detail)


def publish(
    fetch: Fetch,
    run: Runner,
    version: str,
    crates: Sequence[str] = ORDER,
    poll: Poll = Poll(),
    record: Record | None = None,
    sleep: Callable[[float], None] = time.sleep,
    head: str = "",
) -> None:
    """Publish each crate whose version is missing, in dependency order."""
    note = record or _summary
    for crate in crates:
        current = probe(fetch, crate, version)
        if current.state == "unreadable":
            raise ReleaseError(
                f"cannot read {index_url(crate)} ({current.detail}); refusing to publish blind"
            )
        if current.state == "indexed":
            note(f"`{crate}` {version} is already indexed; skipping it")
            continue

        # Packaging is verified here rather than up front: this is the first moment the
        # crate's registry dependencies are resolvable.
        _check(run, ["cargo", "package", "--locked", "-p", crate], f"package {crate}")
        _check(run, ["cargo", "publish", "--locked", "-p", crate], f"publish {crate}")
        if not _await_index(fetch, crate, version, poll, sleep):
            minutes = poll.attempts * poll.delay / 60
            raise ReleaseError(f"{crate} {version} did not appear in the index within {minutes:g} minutes")
        note(f"`{crate}` {version} is indexed")

    if head:
        note(f"published from {head[:7]}")


def _await_index(fetch: Fetch, crate: str, version: str, poll: Poll, sleep: Callable[[float], None]) -> bool:
    for attempt in range(1, poll.attempts + 1):
        if probe(fetch, crate, version).state == "indexed":
            _summary(f"`{crate}` {version} appeared in the index after {attempt} attempt(s)")
            return True
        sleep(poll.delay)
    return False


def dry_run_summary(version: str, probes: Sequence[Probe]) -> str:
    """The dry run's step summary, as one block of markdown."""
    rows = []
    for result in probes:
        if result.state == "indexed":
            action = "skipped"
            state = "already published"
        else:
            action = "packaged, then published"
            state = result.state
        rows.append(f"| `{result.crate}` | {state} | {action} |")

    return "\n".join(
        [
            "### Dry run: nothing was published",
            "",
            f"Packaged and verified `{MACROS}` at `{version}`.",
            "",
            f"`{MAIN}` is not packaged here: it requires `{MACROS}` `{version}` from the",
            "registry, which cannot exist before the first publish. The publish run",
            "packages it immediately after that version is indexed.",
            "",
            f"| crate | {version} on the index | next action in a publish run |",
            "| --- | --- | --- |",
            *rows,
        ]
    )


def tag(run: Runner, version: str, notes_path: Path | str, sha: str) -> str:
    """Create the GitHub Release, unless it already exists.

    An existing release is left untouched: a re-run after a partial publish must not fail
    on something it already achieved.
    """
    tag_name = f"v{version}"
    if run(["gh", "release", "view", tag_name]) == 0:
        return f"{tag_name} already has a release; leaving it untouched"

    path = Path(notes_path)
    if not path.is_file() or path.stat().st_size == 0:
        raise ReleaseError(f"{tag_name} has no verified release notes at {path}; refusing to tag")
    _check(
        run,
        ["gh", "release", "create", tag_name, "--target", sha, "--title", tag_name, "--notes-file", str(path)],
        f"create {tag_name}",
    )
    return f"created {tag_name} with the changelog notes"


def regenerate_changelog(
    root: Path,
    branch: str,
    run: Runner,
    render: Callable[[Path], str],
    commit: Callable[[Path, str], str],
) -> str:
    """Render `CHANGELOG.md` with the tag this run created, then commit and push it.

    The order is the whole point. The tag was created on the remote by the step before,
    and the renderer reads tags, so it is fetched first: rendered without it, the new
    version would come out as `[Unreleased]` again. And the file has to be *written*
    before it is committed — `commit` only commits a file that changed, so skipping the
    render made this step report "already matches the tag" and do nothing, which is
    what the 0.2.0 release did.
    """
    _check(
        run,
        ["git", "fetch", "--tags", "--force", "origin"],
        "fetch the tag that was just created",
    )
    (Path(root) / CHANGELOG).write_text(render(Path(root)), encoding="utf-8")
    return commit(Path(root), branch)


def _render_changelog(root: Path) -> str:
    return generate(convco(root), root)


def preflight(
    root: Path = ROOT,
    ref: str = "",
    output: Path | str | None = None,
    notes_path: Path | str | None = None,
    notes_from: Callable[[str], str] | None = None,
) -> Preflight:
    """Prove everything that must be true before the first irreversible step."""
    if ref and ref != MAIN_REF:
        raise ReleaseError(f"release runs from {MAIN_REF} only; this is {ref}")

    cargo = (root / "Cargo.toml").read_text(encoding="utf-8")
    pixi = (root / "pixi.toml").read_text(encoding="utf-8")
    version = versions(cargo, pixi)[0].value

    problems = [*inconsistencies(cargo, pixi), *crate_pins(root, ORDER)]
    if problems:
        raise ReleaseError("; ".join(problems))

    provider = notes_from or (lambda candidate: notes_for(convco(root), candidate, root=root))
    try:
        notes = provider(version)
    except ChangelogError as error:
        raise ReleaseError(
            f"this release has no notes to attach ({error}). The notes are rendered from the "
            "commits since the last tag, so a release needs commits that describe it."
        ) from error

    path = Path(notes_path) if notes_path else _default_notes_path()
    path.write_text(notes, encoding="utf-8")

    set_output("version", version, path=output)
    set_output("notes_path", str(path), path=output)
    append_summary(f"Release notes for `{version}` resolved ({len(notes.splitlines())} lines)")
    return Preflight(version, notes, path)


def main(
    argv: Sequence[str] | None = None,
    root: Path = ROOT,
    ref: str | None = None,
    output: Path | str | None = None,
    fetch: Fetch | None = None,
    run: Runner | None = None,
    record: Record | None = None,
    sleep: Callable[[float], None] = time.sleep,
    notes_from: Callable[[str], str] | None = None,
    render_changelog: Callable[[Path], str] | None = None,
) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "action", choices=["preflight", "package", "report", "publish", "tag", "changelog"]
    )
    parser.add_argument("crate", nargs="?", default=MACROS, help="the crate to package")
    parser.add_argument("--root", type=Path, default=root, help="the workspace to act on")
    parser.add_argument("--version", help="the version to act on; defaults to the recorded one")
    parser.add_argument("--notes", type=Path, help="the verified notes file, for `tag`")
    parser.add_argument("--branch", default="main", help="the branch `changelog` pushes to")
    arguments = parser.parse_args(argv)

    execute = run or _run
    query = fetch or _fetch
    environment_ref = os.environ.get("GITHUB_REF", "") if ref is None else ref

    try:
        if arguments.action == "preflight":
            result = preflight(
                root=arguments.root, ref=environment_ref, output=output, notes_from=notes_from
            )
            print(f"release version: {result.version}")
            print(f"release notes: {result.notes_path}")
            return 0
        if arguments.action == "package":
            _check(execute, ["cargo", "package", "--locked", "-p", arguments.crate], arguments.crate)
            return 0
        if arguments.action == "report":
            version = arguments.version or _recorded_version(arguments.root)
            results = [probe(query, crate, version) for crate in ORDER]
            _summary(dry_run_summary(version, results))
            return 0
        if arguments.action == "publish":
            version = arguments.version or _recorded_version(arguments.root)
            publish(
                fetch=query,
                run=execute,
                version=version,
                record=record,
                sleep=sleep,
                head=os.environ.get("GITHUB_SHA", ""),
            )
            return 0
        if arguments.action == "changelog":
            _summary(
                regenerate_changelog(
                    arguments.root,
                    arguments.branch,
                    run=execute,
                    render=render_changelog or _render_changelog,
                    # Looked up at call time, so a test can stand in for the push.
                    commit=lambda root, branch: commit_changelog(
                        root, branch, identity=RELEASE_IDENTITY
                    ),
                )
            )
            return 0
        version = arguments.version or _recorded_version(arguments.root)
        notes = arguments.notes or Path(os.environ.get("NOTES_PATH", ""))
        message = tag(
            run=execute,
            version=version,
            notes_path=notes,
            sha=os.environ.get("GITHUB_SHA", ""),
        )
        _summary(message)
        return 0
    except (ReleaseError, ChangelogError) as error:
        set_error(str(error))
        print(f"release: {error}", file=sys.stderr)
        return 1


def _recorded_version(root: Path) -> str:
    return versions(
        (root / "Cargo.toml").read_text(encoding="utf-8"),
        (root / "pixi.toml").read_text(encoding="utf-8"),
    )[0].value


def _default_notes_path() -> Path:
    base = Path(os.environ.get("RUNNER_TEMP") or tempfile.gettempdir())
    return base / "release-notes.md"


def _check(run: Runner, argv: Sequence[str], what: str) -> None:
    code = run(argv)
    if code != 0:
        raise ReleaseError(f"`{' '.join(argv)}` failed (exit {code}) while trying to {what}")


def _summary(line: str) -> None:
    append_summary(line)


def _run(argv: Sequence[str]) -> int:
    return subprocess.run(list(argv), check=False).returncode


def _fetch(url: str, timeout: float = 30.0) -> tuple[int, str]:
    """The sparse index, read without a client library.

    A transport failure is reported as status 0 rather than raised, so the caller's
    "unreadable" path covers "the network is down" and "the index returned 500" alike.
    """
    request = urllib.request.Request(url, headers={"User-Agent": "clifacet-release"})
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:  # noqa: S310
            return response.status, response.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode("utf-8", "replace")
    except (urllib.error.URLError, OSError) as error:
        return 0, str(error)


if __name__ == "__main__":
    sys.exit(main())
