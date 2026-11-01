"""The advisory exception: what it covers, and when it must be removed.

CLI-19 asks for a review that ends one of two ways — the exception goes away, or the
dependency path and the outcome are recorded. Both endings need the same thing: a
statement checked against `Cargo.lock`, so that "review again later" cannot quietly
become "review never".

These tests pin the three ways that statement can rot: the crate disappears and the
exception lingers, the path changes under the documentation, or one of the two
configuration files loses the exception while the crate is still there.
"""

from __future__ import annotations

import pytest

from scripts.check_advisory_exception import (
    ADVISORY,
    CRATE,
    ROOT,
    Finding,
    check,
    dependency_path,
    documented_path,
    exceptions,
    load_lock,
    main,
    render_path,
)

LOCK = {
    "package": [
        {"name": "clifacet-example", "version": "0.1.0", "dependencies": ["rstest-bdd"]},
        {
            "name": "rstest-bdd",
            "version": "0.6.0",
            "dependencies": ["fluent", "i18n-embed"],
        },
        {"name": "fluent", "version": "0.17.0", "dependencies": []},
        {"name": "i18n-embed", "version": "0.16.0", "dependencies": ["i18n-config"]},
        {"name": "i18n-config", "version": "0.4.8", "dependencies": ["basic-toml"]},
        {"name": "basic-toml", "version": "0.1.10", "dependencies": []},
    ]
}

DENY = """
[advisories]
ignore = [{ id = "RUSTSEC-2026-0374", reason = "Test-only rstest-bdd transitively requires unmaintained basic-toml" }]
"""

AUDIT = """
[advisories]
ignore = ["RUSTSEC-2026-0374"]
"""

DOCS = """
## Dependency advisory exception

```
rstest-bdd 0.6.0 → i18n-embed 0.16.0 → i18n-config 0.4.8 → basic-toml 0.1.10
```
"""

PATH = "rstest-bdd 0.6.0 → i18n-embed 0.16.0 → i18n-config 0.4.8 → basic-toml 0.1.10"


@pytest.fixture
def tree(tmp_path):
    """A repository whose four relevant files agree with each other."""
    (tmp_path / "deny.toml").write_text(DENY, encoding="utf-8")
    (tmp_path / ".cargo").mkdir()
    (tmp_path / ".cargo" / "audit.toml").write_text(AUDIT, encoding="utf-8")
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "development.md").write_text(DOCS, encoding="utf-8")
    (tmp_path / "Cargo.lock").write_text(_lock_text(LOCK), encoding="utf-8")
    return tmp_path


def _lock_text(lock: dict) -> str:
    """Render a package list as the part of a Cargo.lock this script reads."""
    lines = []
    for package in lock["package"]:
        lines.append("[[package]]")
        lines.append(f'name = "{package["name"]}"')
        lines.append(f'version = "{package["version"]}"')
        dependencies = package.get("dependencies", [])
        if dependencies:
            lines.append("dependencies = [")
            lines.extend(f' "{dependency}",' for dependency in dependencies)
            lines.append("]")
        lines.append("")
    return "\n".join(lines)


def test_the_path_is_found_through_the_intermediate_crates(tree):
    path = dependency_path(load_lock(tree / "Cargo.lock"), CRATE, "basic-toml")

    assert path is not None
    assert [name for name, _ in path] == ["rstest-bdd", "i18n-embed", "i18n-config", "basic-toml"]


def test_the_shortest_path_wins_when_there_are_several():
    lock = {
        "package": [
            {"name": "root", "version": "0.1.0", "dependencies": ["a", "b"]},
            {"name": "a", "version": "1.0.0", "dependencies": ["target"]},
            {"name": "b", "version": "1.0.0", "dependencies": ["a"]},
            {"name": "target", "version": "1.0.0", "dependencies": []},
        ]
    }

    path = dependency_path(lock, "root", "target")

    assert [name for name, _ in path] == ["root", "a", "target"]


def test_an_unreachable_crate_has_no_path(tree):
    assert dependency_path(load_lock(tree / "Cargo.lock"), CRATE, "no-such-crate") is None


def test_a_dependency_cycle_does_not_hang():
    lock = {
        "package": [
            {"name": "a", "version": "1.0.0", "dependencies": ["b"]},
            {"name": "b", "version": "1.0.0", "dependencies": ["a"]},
        ]
    }

    assert dependency_path(lock, "a", "missing") is None


def test_exceptions_are_read_from_both_files():
    assert exceptions(DENY, AUDIT) == {"deny": {ADVISORY}, "audit": {ADVISORY}}


def test_an_exception_comment_does_not_count_as_an_exception():
    """`audit.toml` explains the exception in a comment; only the list is an exception."""
    commented = '# ignore = ["RUSTSEC-0000-0000"]\n[advisories]\nignore = []\n'

    assert exceptions(commented, AUDIT)["deny"] == set()


def test_the_documented_path_is_read_back_out_of_the_gate_tree(tree):
    assert documented_path((tree / "docs" / "development.md").read_text()) == PATH


def test_the_rendered_path_names_every_crate_and_version(tree):
    assert render_path(dependency_path(load_lock(tree / "Cargo.lock"), CRATE, "basic-toml")) == PATH


def test_a_consistent_tree_has_no_findings(tree):
    assert check(tree) == []


def test_a_missing_exception_in_one_file_is_reported(tree):
    (tree / ".cargo" / "audit.toml").write_text("[advisories]\nignore = []\n", encoding="utf-8")

    findings = check(tree)

    assert [finding.kind for finding in findings] == ["missing-exception"]
    assert "audit.toml" in findings[0].detail


def test_an_exception_without_its_crate_is_reported_as_removable(tree):
    """The tripwire: when the dependency goes, the exception must go with it."""
    lock = {"package": [{"name": "clifacet-example", "version": "0.1.0", "dependencies": []}]}
    (tree / "Cargo.lock").write_text(_lock_text(lock), encoding="utf-8")

    findings = check(tree)

    assert [finding.kind for finding in findings] == ["stale-exception"]
    assert "remove" in findings[0].detail.lower()


def test_a_path_that_no_longer_matches_the_lock_is_reported(tree):
    (tree / "docs" / "development.md").write_text(
        "```\nrstest-bdd 0.6.0 → basic-toml 0.1.10\n```\n", encoding="utf-8"
    )

    findings = check(tree)

    assert [finding.kind for finding in findings] == ["documented-path"]
    assert PATH in findings[0].detail, "the finding should show what the lock actually says"


def test_an_unreachable_crate_with_a_still_documented_path_is_stale_not_silent(tree):
    lock = {"package": [{"name": "clifacet-example", "version": "0.1.0"}]}
    (tree / "Cargo.lock").write_text(_lock_text(lock), encoding="utf-8")
    (tree / "docs" / "development.md").write_text(DOCS, encoding="utf-8")

    kinds = [finding.kind for finding in check(tree)]

    assert kinds == ["stale-exception"], "nothing else is worth reporting once the crate is gone"


def test_the_committed_tree_is_consistent():
    """The review outcome is true of the repository as committed."""
    assert check(ROOT) == []


def test_a_consistent_tree_exits_zero_and_says_the_exception_is_warranted(tree, capsys):
    assert main([], root=tree) == 0

    assert "warranted" in capsys.readouterr().out


def test_an_inconsistent_tree_exits_non_zero_and_says_which_finding(tree, capsys):
    (tree / "docs" / "development.md").write_text("## Dependency advisory exception\n", encoding="utf-8")

    assert main([], root=tree) == 1

    assert "documented-path" in capsys.readouterr().err


def test_print_path_reports_the_chain_from_that_lockfile(tree, capsys):
    assert main(["--print-path"], root=tree) == 0

    assert capsys.readouterr().out.strip() == PATH


def test_print_path_on_the_committed_tree_names_the_unmaintained_crate(capsys):
    assert main(["--print-path"]) == 0

    assert "basic-toml" in capsys.readouterr().out


def test_a_finding_says_what_to_do():
    finding = Finding(kind="stale-exception", detail="basic-toml is gone")

    assert "basic-toml" in str(finding)
    assert finding.kind in str(finding)
