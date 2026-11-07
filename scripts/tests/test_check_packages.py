"""What a published crate is allowed to contain — checked without publishing it.

`release.yml` verifies the notes and the versions before it publishes, but nothing
looked at what cargo actually puts in the tarball. That is the part a release cannot
take back: a crate published without its licence files, or with test fixtures that
were never meant to ship, stays that way for its version's lifetime.

So the rehearsal half of CLI-18 is a contents contract, checked offline:

- the files cargo lists for a crate include what a registry page needs (README, both
  licence files, the source root) and exclude what is repository-only (BDD features,
  the sandbox and its vendored sources, the build directory);
- the metadata a crates.io page shows is present, resolved through workspace
  inheritance, because a missing description is permanent once published.

These tests pin the rules against synthetic file lists; one integration test proves the
rules hold for the real crates, which is what a rehearsal is for.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.check_packages import (
    CRATES,
    REQUIRED_FILES,
    contents,
    main,
    manifest,
    metadata,
    package_files,
    resolve_metadata,
)

ROOT = Path(__file__).resolve().parents[2]


def test_a_required_file_that_is_absent_is_a_finding():
    findings = contents("clifacet", ["Cargo.toml", "src/lib.rs", "README.md"])

    assert [finding.kind for finding in findings] == ["missing-file"]
    assert "clifacet" in findings[0].detail, "the finding has to say which crate is short"
    assert "LICENSE" in findings[0].detail


def test_a_bdd_feature_is_repository_only():
    findings = contents("clifacet", [*REQUIRED_FILES, "features/greeting.feature"])

    assert [finding.kind for finding in findings] == ["forbidden-file"], (
        "a scenario file is test scaffolding; it does not belong in the published crate"
    )


def test_the_sandbox_and_its_vendored_sources_are_never_packaged():
    findings = contents(
        "clifacet", [*REQUIRED_FILES, ".pixi-sandbox/vendor/rstest-bdd-0.6.0/Cargo.toml"]
    )

    assert [finding.kind for finding in findings] == ["forbidden-file"]


def test_a_complete_file_list_has_no_findings():
    assert contents("clifacet", [*REQUIRED_FILES, "src/policy.rs", "tests/version.rs"]) == []


def test_workspace_inherited_metadata_is_resolved_from_the_workspace_table():
    workspace = {"license": "MIT OR Apache-2.0", "rust-version": "1.88"}
    package = {
        "description": "Procedural macros for Clifacet",
        "license": {"workspace": True},
        "rust-version": {"workspace": True},
    }

    resolved = resolve_metadata(package, workspace)

    assert resolved == {
        "description": "Procedural macros for Clifacet",
        "license": "MIT OR Apache-2.0",
        "rust-version": "1.88",
    }


def test_missing_metadata_names_the_field_and_the_crate():
    findings = metadata("clifacet", {"description": "x"}, {})

    assert [finding.kind for finding in findings] == ["missing-metadata"]
    assert "clifacet" in findings[0].detail and "license" in findings[0].detail


def test_inherited_metadata_with_nothing_to_inherit_is_reported_not_guessed():
    declared = {"description": "x", "license": {"workspace": True}, "repository": "r", "rust-version": "1.88"}
    findings = metadata("clifacet", declared, {})

    assert [finding.kind for finding in findings] == ["unresolvable-metadata"]


def test_package_files_reads_cargos_listing(tmp_path):
    seen = {}

    def run(command, root):
        seen["command"] = command
        return "Cargo.toml\nsrc/lib.rs\n"

    files = package_files("clifacet-macros", root=tmp_path, run=run)

    assert files == ["Cargo.toml", "src/lib.rs"]
    assert seen["command"][:4] == ["cargo", "package", "--list", "--locked"]
    assert "clifacet-macros" in seen["command"]


def test_package_files_ignores_cargos_status_lines(tmp_path):
    def run(command, root):
        return "   Packaging clifacet-macros v0.1.0\nCargo.toml\n\nsrc/lib.rs\n"

    assert package_files("clifacet-macros", root=tmp_path, run=run) == ["Cargo.toml", "src/lib.rs"]


def test_manifest_reads_a_crate_manifest_and_its_workspace_table(tmp_path):
    (tmp_path / "Cargo.toml").write_text(
        '[workspace.package]\nlicense = "MIT OR Apache-2.0"\n', encoding="utf-8"
    )
    crate = tmp_path / "crates" / "clifacet-macros"
    crate.mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        '[package]\nname = "clifacet-macros"\nlicense.workspace = true\n', encoding="utf-8"
    )

    package, workspace = manifest(tmp_path, "clifacet-macros")

    assert package["name"] == "clifacet-macros"
    assert workspace["license"] == "MIT OR Apache-2.0"


def test_the_packaged_licence_texts_are_copies_of_the_root_ones():
    for crate in CRATES:
        for name in ("LICENSE-MIT", "LICENSE-APACHE"):
            shipped = ROOT / "crates" / crate / name
            assert shipped.is_file(), f"{crate} must ship {name} inside its package root"

            assert shipped.read_bytes() == (ROOT / name).read_bytes(), (
                f"crates/{crate}/{name} has drifted from the root copy"
            )


def test_the_real_crates_pass_the_contract():
    findings = [finding for crate in CRATES for finding in _check_one(crate)]

    assert findings == [], "a rehearsal that fails here is what a release would ship"


def test_main_reports_success_for_the_real_workspace(capsys):
    assert main([]) == 0

    out = capsys.readouterr().out
    assert "clifacet-macros" in out and "clifacet" in out


@pytest.mark.parametrize("crate", CRATES)
def test_the_publish_order_starts_with_the_macros(crate):
    assert CRATES[0] == "clifacet-macros", (
        "the main crate cannot resolve its macro requirement before that version is indexed"
    )


def _check_one(crate):
    from scripts.check_packages import check

    return check(crates=(crate,))
