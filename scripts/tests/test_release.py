"""The release workflow's decisions, exercised without a registry or a dispatch.

Everything here used to live in `release.yml`, where the only way to find out whether it
worked was to dispatch a run — and the answers that matter most are the ones a dispatch
gives *once*: what happens when a crate is already indexed, when the index cannot be
read, when the version never appears, and when a run dies between the two publishes.

The seams are the two effects the workflow has: `fetch(url) -> (status, body)` for the
sparse index, and `run(argv) -> exit code` for cargo, git and gh. Everything else is a
pure function over those results, so the ordering rules are asserted as ordering rules.
"""

from __future__ import annotations

import pytest

from scripts.release import (
    MAIN,
    MACROS,
    ORDER,
    Poll,
    Probe,
    ReleaseError,
    dry_run_summary,
    index_url,
    main,
    preflight,
    probe,
    publish,
    tag,
    versions_in,
)

VERSION = "0.1.0"
INDEXED = (200, '{"name":"clifacet","vers":"0.0.9"}\n{"name":"clifacet","vers":"0.1.0"}\n')
ABSENT = (200, '{"name":"clifacet","vers":"0.0.9"}\n')
MISSING = (404, "not found")
BROKEN = (503, "service unavailable")


def fetch_returning(table):
    """A fetch that answers from a `{crate: (status, body)}` table and records calls."""
    calls = []

    def fetch(url):
        calls.append(url)
        crate = url.rsplit("/", 1)[-1]
        return table[crate]

    fetch.calls = calls
    return fetch


def indexing_fetch(table, events):
    """An index that reports `absent` until the crate is published, then `indexed`.

    The real index behaves this way, and it is what makes the poll terminate.
    """
    def fetch(url):
        crate = url.rsplit("/", 1)[-1]
        published = {argv[-1] for argv in events if argv[1:2] == ["publish"]}
        return INDEXED if crate in published else table[crate]

    return fetch


def runner(events, exit_code=0):
    """A runner that records commands and reports success unless told otherwise."""
    def run(argv):
        events.append(list(argv))
        code = exit_code(argv) if callable(exit_code) else exit_code
        return code

    return run


# --- the index ---------------------------------------------------------------


def test_the_index_url_uses_the_two_two_prefix_layout():
    assert index_url("clifacet") == "https://index.crates.io/cl/if/clifacet"


def test_versions_are_read_from_the_index_entry():
    assert versions_in(INDEXED[1]) == {"0.0.9", "0.1.0"}


def test_a_present_version_is_indexed():
    assert probe(fetch_returning({MAIN: INDEXED}), MAIN, VERSION).state == "indexed"


def test_a_crate_without_this_version_is_absent():
    assert probe(fetch_returning({MAIN: ABSENT}), MAIN, VERSION).state == "absent"


def test_a_crate_that_was_never_published_is_absent_rather_than_an_error():
    assert probe(fetch_returning({MAIN: MISSING}), MAIN, VERSION).state == "absent"


def test_a_failing_index_read_is_unreadable_not_absent():
    """The distinction matters: absent means publish, unreadable means stop."""
    result = probe(fetch_returning({MAIN: BROKEN}), MAIN, VERSION)

    assert result.state == "unreadable"
    assert "503" in result.detail


# --- publishing --------------------------------------------------------------


def test_an_indexed_version_is_skipped_so_a_rerun_finishes_a_partial_release():
    events, recorded = [], []

    publish(
        fetch=fetch_returning({MACROS: INDEXED, MAIN: INDEXED}),
        run=runner(events),
        version=VERSION,
        record=recorded.append,
    )

    assert events == [], "nothing is packaged or published for a version already on the index"
    assert any("already indexed" in line for line in recorded)


def test_an_absent_version_is_packaged_then_published():
    events = []
    table = {MACROS: ABSENT}

    def fetch(url):
        crate = url.rsplit("/", 1)[-1]
        # The publish is what makes the version appear on the index.
        return INDEXED if events else table[crate]

    publish(
        fetch=fetch,
        run=runner(events),
        version=VERSION,
        crates=(MACROS,),
        record=lambda line: None,
        sleep=lambda seconds: None,
    )

    assert events == [
        ["cargo", "package", "--locked", "-p", MACROS],
        ["cargo", "publish", "--locked", "-p", MACROS],
    ]


def test_the_macro_crate_is_published_before_the_main_crate():
    """The main crate's registry requirement cannot resolve until the macro version exists."""
    events = []

    publish(
        fetch=indexing_fetch({MACROS: ABSENT, MAIN: ABSENT}, events),
        run=lambda argv: (events.append(list(argv)), 0)[1],
        version=VERSION,
        record=lambda line: None,
        poll=Poll(attempts=2, delay=0),
        sleep=lambda seconds: None,
    )

    published = [argv[-1] for argv in events if argv[1] == "publish"]
    assert published == [MACROS, MAIN]
    assert ORDER == (MACROS, MAIN)


def test_an_unreadable_index_refuses_to_publish_blind():
    events = []

    with pytest.raises(ReleaseError, match="refusing"):
        publish(
            fetch=fetch_returning({MACROS: BROKEN}),
            run=runner(events),
            version=VERSION,
            crates=(MACROS,),
            record=lambda line: None,
        )

    assert events == [], "a failed index read must not fall through to a publish"


def test_the_index_is_polled_until_the_new_version_appears():
    polls = {"n": 0}

    def fetch(url):
        polls["n"] += 1
        return (200, ABSENT[1]) if polls["n"] < 3 else INDEXED

    publish(
        fetch=fetch,
        run=runner([]),
        version=VERSION,
        crates=(MACROS,),
        record=lambda line: None,
        poll=Poll(attempts=5, delay=0),
        sleep=lambda seconds: None,
    )

    assert polls["n"] == 3


def test_a_version_that_never_appears_fails_the_run():
    with pytest.raises(ReleaseError, match="did not appear in the index"):
        publish(
            fetch=fetch_returning({MACROS: ABSENT}),
            run=runner([]),
            version=VERSION,
            crates=(MACROS,),
            record=lambda line: None,
            poll=Poll(attempts=2, delay=0),
            sleep=lambda seconds: None,
        )


def test_a_failed_cargo_publish_names_the_crate_and_stops():
    events = []

    def exit_code(argv):
        return 1 if argv[1] == "publish" else 0

    with pytest.raises(ReleaseError, match=MACROS):
        publish(
            fetch=fetch_returning({MACROS: ABSENT, MAIN: ABSENT}),
            run=runner(events, exit_code),
            version=VERSION,
            record=lambda line: None,
        )

    assert not any(argv[1] == "publish" and argv[-1] == MAIN for argv in events), (
        "the main crate must not be published after the macro crate failed"
    )


def test_a_partial_release_is_finished_by_rerunning_the_same_command():
    """The whole recovery story: the macro is on the index, the main crate is not."""
    events, recorded = [], []

    publish(
        fetch=indexing_fetch({MACROS: INDEXED, MAIN: ABSENT}, events),
        run=runner(events),
        version=VERSION,
        record=recorded.append,
        poll=Poll(attempts=2, delay=0),
        sleep=lambda seconds: None,
    )

    assert [argv[-1] for argv in events if argv[1] == "publish"] == [MAIN]
    assert any("already indexed" in line for line in recorded)


# --- the dry-run report ------------------------------------------------------


def test_the_dry_run_report_tabulates_both_crates():
    summary = dry_run_summary(
        VERSION,
        [Probe(MACROS, VERSION, "indexed"), Probe(MAIN, VERSION, "absent")],
    )

    assert "nothing was published" in summary
    assert f"| `{MACROS}` | already published | skipped |" in summary
    assert f"| `{MAIN}` | absent | packaged, then published |" in summary


def test_the_dry_run_report_says_why_the_main_crate_was_not_packaged():
    summary = dry_run_summary(VERSION, [Probe(MAIN, VERSION, "absent")])

    assert "cannot exist before the first publish" in summary


# --- tagging -----------------------------------------------------------------


def test_a_release_that_already_exists_is_left_alone():
    events = []

    def run(argv):
        events.append(list(argv))
        return 0  # `gh release view` succeeded

    message = tag(run=run, version=VERSION, notes_path="notes.md", sha="abc123")

    assert "leaving it untouched" in message
    assert not any(argv[:3] == ["gh", "release", "create"] for argv in events)


def test_the_release_is_created_with_the_verified_notes_and_the_commit(tmp_path):
    events = []
    notes = tmp_path / "notes.md"
    notes.write_text("### Added\n\n- a thing\n", encoding="utf-8")

    def run(argv):
        events.append(list(argv))
        return 1 if argv[2] == "view" else 0  # nothing to view; the create succeeds

    tag(run=run, version=VERSION, notes_path=notes, sha="abc123")

    create = next(argv for argv in events if argv[:3] == ["gh", "release", "create"])
    assert create == [
        "gh", "release", "create", f"v{VERSION}",
        "--target", "abc123", "--title", f"v{VERSION}",
        "--notes-file", str(notes),
    ]


def test_empty_notes_are_refused_rather_than_tagged(tmp_path):
    notes = tmp_path / "notes.md"
    notes.write_text("", encoding="utf-8")

    with pytest.raises(ReleaseError, match="notes"):
        tag(run=lambda argv: 1 if argv[2] == "view" else 0, version=VERSION, notes_path=notes, sha="abc123")


# --- preflight ---------------------------------------------------------------

CARGO = """[workspace.package]
version = "0.1.0"

[workspace.dependencies]
clifacet-macros = { version = "0.1.0", path = "crates/clifacet-macros" }
"""

PIXI = """[workspace]
name = "clifacet"
version = "0.1.0"
"""

CHANGELOG = """# Changelog

## [Unreleased]

## [0.1.0] - 2026-11-01

### Added

- The first release.
"""


def release_tree(tmp_path, changelog=CHANGELOG, macro_version_inherits=True):
    """A tree shaped like the repository as far as the release job cares."""
    (tmp_path / "Cargo.toml").write_text(CARGO, encoding="utf-8")
    (tmp_path / "pixi.toml").write_text(PIXI, encoding="utf-8")
    (tmp_path / "CHANGELOG.md").write_text(changelog, encoding="utf-8")
    for crate in (MACROS, MAIN):
        directory = tmp_path / "crates" / crate
        directory.mkdir(parents=True)
        inherited = "version.workspace = true" if macro_version_inherits else 'version = "0.0.9"'
        (directory / "Cargo.toml").write_text(
            f'[package]\nname = "{crate}"\n{inherited}\n', encoding="utf-8"
        )
    return tmp_path


def test_preflight_reads_the_version_the_manifests_record(tmp_path):
    result = preflight(root=release_tree(tmp_path), ref="refs/heads/main")

    assert result.version == VERSION
    assert "The first release." in result.notes


def test_preflight_writes_the_outputs_the_later_steps_need(tmp_path):
    output = tmp_path / "output"

    preflight(root=release_tree(tmp_path), ref="refs/heads/main", output=output)

    text = output.read_text(encoding="utf-8")
    assert "version=0.1.0" in text
    assert "notes_path=" in text


def test_preflight_refuses_a_ref_that_is_not_main(tmp_path):
    with pytest.raises(ReleaseError, match="main only"):
        preflight(root=release_tree(tmp_path), ref="refs/heads/some-branch")


def test_preflight_explains_how_to_make_the_notes_readable(tmp_path):
    """The failure the first rehearsal actually produced."""
    tree = release_tree(tmp_path, changelog="# Changelog\n\n## [Unreleased]\n\n### Added\n\n- x\n")

    with pytest.raises(ReleaseError) as failure:
        preflight(root=tree, ref="refs/heads/main")

    assert "--promote-current" in str(failure.value)


def test_preflight_refuses_a_crate_that_pins_its_own_version(tmp_path):
    """Crates inherit the workspace version; a pin silently ships the wrong one."""
    tree = release_tree(tmp_path, macro_version_inherits=False)

    with pytest.raises(ReleaseError, match=MACROS):
        preflight(root=tree, ref="refs/heads/main")


def test_preflight_refuses_version_sites_that_disagree(tmp_path):
    tree = release_tree(tmp_path)
    (tree / "pixi.toml").write_text('[workspace]\nversion = "0.2.0"\n', encoding="utf-8")

    with pytest.raises(ReleaseError, match="pixi.toml"):
        preflight(root=tree, ref="refs/heads/main")


def test_preflight_without_a_ref_runs_locally(tmp_path):
    """No GITHUB_REF means a developer running it by hand, not a dispatch."""
    assert preflight(root=release_tree(tmp_path), ref="").version == VERSION


# --- the command the workflow calls -----------------------------------------


def test_main_reports_a_preflight_failure_without_a_traceback(tmp_path, capsys):
    tree = release_tree(tmp_path)
    (tree / "pixi.toml").write_text('[workspace]\nversion = "0.2.0"\n', encoding="utf-8")

    assert main(["preflight", "--root", str(tree)], ref="refs/heads/main") == 1
    assert "::error::" in capsys.readouterr().err


def test_main_preflight_prints_the_version_it_resolved(tmp_path, capsys):
    tree = release_tree(tmp_path)

    assert main(["preflight", "--root", str(tree)], ref="refs/heads/main", output=tmp_path / "out") == 0
    assert "release version: 0.1.0" in capsys.readouterr().out
