"""Version bumping: validation, the three manifest sites, and changelog promotion."""

from __future__ import annotations

import pytest

from scripts.bump_version import (
    CARGO_MANIFEST as CARGO_PATH,
    CHANGELOG as CHANGELOG_PATH,
    PIXI_MANIFEST as PIXI_PATH,
    bump,
    changelog_notes,
    inconsistencies,
    main,
    parse_version,
    promote_changelog,
    versions,
)

def _read(path):
    return path.read_text(encoding="utf-8")


CARGO = """[workspace]
members = ["crates/clifacet", "crates/clifacet-macros"]
resolver = "3"

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.dependencies]
clifacet-macros = { version = "0.1.0", path = "crates/clifacet-macros" }
serde = "1.0.229"
"""

PIXI = """[workspace]
name = "clifacet"
version = "0.1.0"
platforms = ["linux-64"]

[dependencies]
alint = "0.20.1"
"""

CHANGELOG = """# Changelog

## [Unreleased]

### Added

- A feature.

## [0.0.1] - 2026-01-01

### Added

- Something older.

[Unreleased]: https://github.com/Archont561/clifacet/commits/main
[0.0.1]: https://github.com/Archont561/clifacet/releases/tag/v0.0.1
"""


@pytest.mark.parametrize("value", ["0.2.0", "1.0.0", "10.20.30", "1.0.0-rc.1", "0.1.0+build.5"])
def test_valid_versions_are_accepted(value):
    assert parse_version(value) == value


@pytest.mark.parametrize("value", ["", "1", "1.2", "v1.2.3", "1.2.3.4", "01.2.3", "1.2.x", " 1.2.3 x"])
def test_invalid_versions_are_rejected(value):
    with pytest.raises(ValueError, match="not a semver"):
        parse_version(value)


def test_versions_reads_all_three_sites():
    sites = versions(CARGO, PIXI)

    assert [site.value for site in sites] == ["0.1.0", "0.1.0", "0.1.0"]
    assert sites[1].label.endswith("clifacet-macros")


def test_a_drifting_macro_requirement_is_reported():
    cargo = CARGO.replace('clifacet-macros = { version = "0.1.0"', 'clifacet-macros = { version = "0.0.9"')

    problems = inconsistencies(cargo, PIXI)

    assert len(problems) == 1
    assert "0.0.9" in problems[0]


def test_a_drifting_pixi_version_is_reported():
    problems = inconsistencies(CARGO, PIXI.replace('version = "0.1.0"', 'version = "0.3.0"', 1))

    assert len(problems) == 1
    assert "pixi.toml" in problems[0]


def test_consistent_manifests_report_nothing():
    assert inconsistencies(CARGO, PIXI) == []


def test_bump_rewrites_every_site():
    cargo, pixi = bump(CARGO, PIXI, "0.2.0")

    assert [site.value for site in versions(cargo, pixi)] == ["0.2.0", "0.2.0", "0.2.0"]


def test_bump_leaves_unrelated_versions_untouched():
    cargo, pixi = bump(CARGO, PIXI, "0.2.0")

    assert 'serde = "1.0.229"' in cargo
    assert 'alint = "0.20.1"' in pixi
    assert 'edition = "2024"' in cargo
    assert 'clifacet-macros = { version = "0.2.0", path = "crates/clifacet-macros" }' in cargo


def test_bump_keeps_the_rest_of_each_file_byte_identical():
    cargo, pixi = bump(CARGO, PIXI, "0.2.0")

    assert len(cargo.splitlines()) == len(CARGO.splitlines())
    assert len(pixi.splitlines()) == len(PIXI.splitlines())


def test_a_manifest_without_a_version_section_is_an_error():
    with pytest.raises(ValueError, match="no \\[workspace.package\\] section"):
        bump(CARGO.replace("[workspace.package]", "[workspace.metadata]"), PIXI, "0.2.0")


def test_a_manifest_without_a_macro_requirement_is_an_error():
    with pytest.raises(ValueError, match="clifacet-macros"):
        bump(CARGO.replace('clifacet-macros = { version = "0.1.0", path = "crates/clifacet-macros" }\n', ""), PIXI, "0.2.0")


def test_promote_turns_unreleased_into_the_release_and_reopens_it():
    promoted = promote_changelog(CHANGELOG, "0.1.0", "2026-10-30")

    assert "## [0.1.0] - 2026-10-30" in promoted
    assert promoted.count("## [Unreleased]") == 1
    assert promoted.index("## [Unreleased]") < promoted.index("## [0.1.0] - 2026-10-30")
    assert "- A feature." in promoted


def test_promote_moves_the_unreleased_link_to_the_new_tag():
    promoted = promote_changelog(CHANGELOG, "0.1.0", "2026-10-30")

    assert "[Unreleased]: https://github.com/Archont561/clifacet/compare/v0.1.0...HEAD" in promoted
    assert "[0.1.0]: https://github.com/Archont561/clifacet/releases/tag/v0.1.0" in promoted


def test_promote_refuses_an_empty_unreleased_section():
    empty = CHANGELOG.replace("### Added\n\n- A feature.\n", "")

    with pytest.raises(ValueError, match="is empty"):
        promote_changelog(empty, "0.1.0", "2026-10-30")


def test_promote_refuses_a_version_that_already_exists():
    with pytest.raises(ValueError, match="already has"):
        promote_changelog(CHANGELOG, "0.0.1", "2026-10-30")


def test_changelog_notes_returns_the_section_body():
    assert changelog_notes(CHANGELOG, "0.0.1").strip() == "### Added\n\n- Something older."


def test_changelog_notes_names_the_versions_it_found():
    with pytest.raises(ValueError, match="found: Unreleased, 0.0.1"):
        changelog_notes(CHANGELOG, "9.9.9")


def test_main_prints_the_current_version(capsys):
    assert main(["--print-version"]) == 0
    assert capsys.readouterr().out.strip() == "0.1.0"


def test_main_check_passes_on_the_committed_manifests(capsys):
    assert main(["--check"]) == 0
    assert "all three version sites record 0.1.0" in capsys.readouterr().out


def test_main_dry_run_reports_the_change_without_writing(capsys):
    manifests = [_read(path) for path in (CARGO_PATH, PIXI_PATH, CHANGELOG_PATH)]

    assert main(["0.2.0", "--dry-run"]) == 0
    assert "would record 0.2.0" in capsys.readouterr().out
    assert [_read(path) for path in (CARGO_PATH, PIXI_PATH, CHANGELOG_PATH)] == manifests


def test_main_rejects_bumping_to_the_current_version(capsys):
    assert main(["0.1.0"]) == 1
    assert "already the recorded version" in capsys.readouterr().err


def test_main_requires_a_version_when_not_asking_a_question():
    with pytest.raises(SystemExit):
        main([])


def test_main_reports_a_missing_notes_section(capsys):
    # 0.1.0 has not been released yet, so it exists only as Unreleased.
    assert main(["--notes", "0.1.0"]) == 1
    assert "no '## [0.1.0]' section" in capsys.readouterr().err


def test_main_prints_notes_for_a_released_version(capsys):
    assert main(["--notes", "0.0.1"]) == 1  # the committed changelog has no such section
    assert "found:" in capsys.readouterr().err


# --- the first release: the recorded version has no dated section yet -------


def _copy_release_files(tmp_path, monkeypatch):
    """Point `main()` at copies of the three files it writes."""
    import scripts.bump_version as bump_version

    cargo = tmp_path / "Cargo.toml"
    pixi = tmp_path / "pixi.toml"
    changelog = tmp_path / "CHANGELOG.md"
    cargo.write_text(_read(CARGO_PATH), encoding="utf-8")
    pixi.write_text(_read(PIXI_PATH), encoding="utf-8")
    changelog.write_text(_read(CHANGELOG_PATH), encoding="utf-8")
    monkeypatch.setattr(bump_version, "CARGO_MANIFEST", cargo)
    monkeypatch.setattr(bump_version, "PIXI_MANIFEST", pixi)
    monkeypatch.setattr(bump_version, "CHANGELOG", changelog)
    return cargo, pixi, changelog


def test_promote_current_dates_the_unreleased_section(tmp_path, monkeypatch):
    cargo, pixi, changelog = _copy_release_files(tmp_path, monkeypatch)
    manifests = (cargo.read_text(encoding="utf-8"), pixi.read_text(encoding="utf-8"))

    assert main(["--promote-current"]) == 0

    text = changelog.read_text(encoding="utf-8")
    assert "## [0.1.0] - " in text, "the recorded version became the dated section"
    assert text.index("## [Unreleased]") < text.index("## [0.1.0] - ")
    assert (
        cargo.read_text(encoding="utf-8"),
        pixi.read_text(encoding="utf-8"),
    ) == manifests, "promoting the changelog is not a version bump"


def test_promote_current_makes_the_notes_readable(tmp_path, monkeypatch, capsys):
    _, _, _ = _copy_release_files(tmp_path, monkeypatch)

    assert main(["--promote-current"]) == 0
    capsys.readouterr()

    # What `release.yml` asks for, after the release pull request has been merged.
    assert main(["--notes", "0.1.0"]) == 0
    assert capsys.readouterr().out.strip(), "the tag step now has notes to attach"


def test_promote_current_dry_run_reports_without_writing(tmp_path, monkeypatch, capsys):
    _, _, changelog = _copy_release_files(tmp_path, monkeypatch)
    before = changelog.read_text(encoding="utf-8")

    assert main(["--promote-current", "--dry-run"]) == 0

    assert "would promote" in capsys.readouterr().out
    assert changelog.read_text(encoding="utf-8") == before


def test_promote_current_refuses_a_version_that_is_already_released(tmp_path, monkeypatch, capsys):
    _, _, changelog = _copy_release_files(tmp_path, monkeypatch)
    # The state just after a release: Unreleased was reopened, and it is empty.
    changelog.write_text("# Changelog\n\n## [Unreleased]\n", encoding="utf-8")

    assert main(["--promote-current"]) == 1
    assert "empty" in capsys.readouterr().err


def test_promote_current_refuses_when_the_section_already_exists(tmp_path, monkeypatch, capsys):
    _, _, changelog = _copy_release_files(tmp_path, monkeypatch)
    main(["--promote-current"])
    capsys.readouterr()

    assert main(["--promote-current"]) == 1
    assert "already has" in capsys.readouterr().err
