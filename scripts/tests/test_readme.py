"""The README is rendered, and the renderer is the only thing that knows how.

Three claims are checked here, and each one stands for a way the README has already been
got wrong:

- **The template is the source.** `README.md` is generated from
  `.readme/README.template.md` by `readme-md-generator`, an npm tool. If the two disagree,
  somebody edited the rendered file, and the next render silently discards the edit.
- **The values are the repository's.** The generator reads a `package.json`, and this
  repository's describes the development tooling — name `clifacet-dev`, version `0.1.0`,
  while the crate is `clifacet` at whatever `Cargo.toml` says. A context built from it
  would put the wrong name in the first line and the wrong version in the install snippet.
- **The tool is invoked correctly.** The generator prompts before overwriting an existing
  README and has no flag to skip the prompt, so the render has to happen in a directory
  where no README exists. A renderer that ran it in the repository root would hang.

The generator itself is not run here: it needs the network on a cold cache, and a test
that fails for the network teaches nothing about the README. `main` is exercised with the
render substituted, which is where the decisions live.
"""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

import pytest

from scripts import readme

ROOT = Path(__file__).resolve().parents[2]


def test_the_context_names_the_crate_not_the_tooling_package():
    """`package.json` says `clifacet-dev`; the README must say `clifacet`."""
    context = readme.context()
    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))

    assert context["name"] == "clifacet"
    assert context["name"] != package["name"], (
        "the tooling package's name is not the crate's name; deriving one from the other "
        "is how the README came to be about the wrong project"
    )


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


def test_the_template_exists_and_belongs_to_the_generator():
    assert readme.TEMPLATE.is_file()
    assert readme.TEMPLATE.name == "README.template.md"
    assert readme.TEMPLATE.parent.name == ".readme"


def test_the_generator_is_pinned_to_a_version():
    """A name installs whatever is newest, so the README could change with no commit."""
    assert readme.GENERATOR.startswith("readme-md-generator@")
    assert len(readme.GENERATOR.partition("@")[2]) > 0


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


def test_a_missing_bun_is_reported_rather_than_crashing(monkeypatch):
    """The renderer runs through Pixi; without it the message should say so."""
    monkeypatch.setattr(readme.shutil, "which", lambda _: None)

    with pytest.raises(SystemExit) as exit_info:
        readme.bun()

    assert "pixi run readme" in str(exit_info.value)
