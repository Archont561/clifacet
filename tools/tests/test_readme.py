"""The README is rendered from templates/README.template.md.

The template is the source: `README.md` is generated from
`templates/README.template.md`. If the two disagree, somebody edited the rendered
file, and the next render silently discards the edit.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from tools._lib import readme

ROOT = Path(__file__).resolve().parents[2]


def test_the_context_names_the_crate():
    """The README must say `clifacet`."""
    context = readme.context()
    assert context["name"] == "clifacet"


def test_the_context_version_is_the_workspace_version():
    """The version a reader installs is the one `bump_version.py` records."""
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    assert readme.context()["version"] == manifest["workspace"]["package"]["version"]


def test_the_context_repository_is_the_published_one():
    """A checkout's remote may be a fork or an SSH alias; the manifest is canonical."""
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    assert readme.context()["repository"]["url"] == manifest["workspace"]["package"]["repository"]


def test_the_author_is_a_name_rather_than_a_name_and_address():
    """The template prints the author; `Name <address>` would print the address too."""
    author = readme.context()["author"]
    assert author
    assert "<" not in author


def test_the_template_exists():
    assert readme.TEMPLATE.is_file()
    assert readme.TEMPLATE.name == "README.template.md"
    assert readme.TEMPLATE.parent.name == "templates"


def test_rendering_substitutes_all_placeholders():
    rendered = readme.render()
    assert "<%=" not in rendered
    assert "%>" not in rendered
    manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    assert manifest["workspace"]["package"]["version"] in rendered


def test_check_passes_when_the_readme_is_what_the_template_renders(monkeypatch, capsys, tmp_path):
    rendered = "# clifacet\n\nA rendered README.\n"
    target = tmp_path / "README.md"
    target.write_text(rendered, encoding="utf-8")
    monkeypatch.setattr(readme, "render", lambda: rendered)
    monkeypatch.setattr(readme, "README", target)

    assert readme.main(["--check"]) == 0
    assert "matches the template" in capsys.readouterr().out


def test_check_fails_when_the_readme_was_edited_by_hand(monkeypatch, capsys, tmp_path):
    target = tmp_path / "README.md"
    target.write_text("edited by hand\n", encoding="utf-8")
    monkeypatch.setattr(readme, "render", lambda: "# clifacet\n")
    monkeypatch.setattr(readme, "README", target)

    assert readme.main(["--check"]) == 1
    assert "not what" in capsys.readouterr().err


def test_writing_records_the_rendered_text(monkeypatch, tmp_path):
    rendered = "# clifacet\n\nA rendered README.\n"
    target = tmp_path / "README.md"
    monkeypatch.setattr(readme, "render", lambda: rendered)
    monkeypatch.setattr(readme, "README", target)

    assert readme.main([]) == 0
    assert target.read_text(encoding="utf-8") == rendered


def test_rendering_without_the_template_says_which_file_is_missing(monkeypatch, tmp_path):
    """A deleted template is the one failure the renderer can name precisely."""
    monkeypatch.setattr(readme, "TEMPLATE", tmp_path / "absent.md")

    with pytest.raises(SystemExit) as exit_info:
        readme.render()

    assert "template is missing" in str(exit_info.value)
