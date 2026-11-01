"""Version bumping: validation, and the three manifest sites that record a version."""

from __future__ import annotations

import pytest

from scripts.bump_version import (
    bump,
    crate_pins,
    inconsistencies,
    main,
    parse_version,
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


def test_a_crate_that_inherits_the_workspace_version_is_not_a_pin(tmp_path):
    crate = tmp_path / "crates" / "clifacet-macros"
    crate.mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "clifacet-macros"\nversion.workspace = true\n', encoding="utf-8"
    )

    assert crate_pins(tmp_path, ["clifacet-macros"]) == []


def test_a_crate_that_declares_its_own_version_is_reported(tmp_path):
    """The workspace version is the one place a release reads; a pin overrides it silently."""
    crate = tmp_path / "crates" / "clifacet-macros"
    crate.mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "clifacet-macros"\nversion = "0.0.9"\n', encoding="utf-8"
    )

    problems = crate_pins(tmp_path, ["clifacet-macros"])

    assert len(problems) == 1
    assert "clifacet-macros" in problems[0] and "0.0.9" in problems[0]


def test_a_missing_crate_manifest_is_reported_not_ignored(tmp_path):
    assert crate_pins(tmp_path, ["clifacet-macros"]), "an unreadable manifest cannot be assumed consistent"


def test_main_prints_the_current_version(tmp_path, monkeypatch, capsys):
    _install_fixtures(tmp_path, monkeypatch)

    assert main(["--print-version"]) == 0
    assert capsys.readouterr().out.strip() == "0.1.0"


def test_main_check_passes_on_the_committed_manifests(capsys):
    assert main(["--check"]) == 0
    assert "all three version sites record 0.1.0" in capsys.readouterr().out


def test_main_dry_run_reports_the_change_without_writing(tmp_path, monkeypatch, capsys):
    files = _install_fixtures(tmp_path, monkeypatch)
    before = [_read(path) for path in files]

    assert main(["0.2.0", "--dry-run"]) == 0
    assert "would record 0.2.0" in capsys.readouterr().out
    assert [_read(path) for path in files] == before


def test_main_writes_every_site(tmp_path, monkeypatch, capsys):
    cargo, pixi = _install_fixtures(tmp_path, monkeypatch)

    assert main(["0.2.0"]) == 0
    assert [site.value for site in versions(_read(cargo), _read(pixi))] == ["0.2.0"] * 3


def test_main_does_not_touch_the_changelog(tmp_path, monkeypatch):
    """The changelog is rendered from the commits; a bump that edited it could disagree."""
    files = _install_fixtures(tmp_path, monkeypatch)
    changelog = tmp_path / "CHANGELOG.md"
    before = changelog.read_text(encoding="utf-8")

    assert main(["0.2.0"]) == 0

    assert changelog.read_text(encoding="utf-8") == before
    assert len(files) == 2, "the bump knows about two manifests and nothing else"


def test_main_rejects_bumping_to_the_current_version(capsys):
    assert main(["0.1.0"]) == 1
    assert "already the recorded version" in capsys.readouterr().err


def test_main_requires_a_version_when_not_asking_a_question():
    with pytest.raises(SystemExit):
        main([])


def _install_fixtures(tmp_path, monkeypatch):
    """Point `main()` at fixture copies of the two manifests it reads and writes."""
    import scripts.bump_version as bump_version

    cargo = tmp_path / "Cargo.toml"
    pixi = tmp_path / "pixi.toml"
    cargo.write_text(CARGO, encoding="utf-8")
    pixi.write_text(PIXI, encoding="utf-8")
    changelog = tmp_path / "CHANGELOG.md"
    changelog.write_text("# Changelog\n\n## [Unreleased]\n\n- A feature.\n", encoding="utf-8")
    monkeypatch.setattr(bump_version, "CARGO_MANIFEST", cargo)
    monkeypatch.setattr(bump_version, "PIXI_MANIFEST", pixi)
    return cargo, pixi
