"""The documentation contract: links resolve, snippets compile, API is real.

Three claims are made about the guides, and none of them can be proven by reading
them: that a link goes somewhere, that a code block is the code that actually
exists, and that the names in it were not invented. Each is a mechanical
question, so each is asked here and answered by `scripts/check_docs.py`.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from scripts.check_docs import (
    SECTIONS,
    Finding,
    api_paths,
    check,
    check_staleness,
    uncategorised,
    generated_snippets,
    headings,
    links,
    rust_blocks,
    slug,
)

ROOT = Path(__file__).resolve().parents[2]


@pytest.fixture
def docs(tmp_path):
    """A miniature documentation tree: one index and one guide that links into it."""
    (tmp_path / "docs" / "guides").mkdir(parents=True)
    (tmp_path / "README.md").write_text(
        "# Project\n\nSee the [guide](docs/guides/guide.md#using-it) and [the site](https://example.com).\n",
        encoding="utf-8",
    )
    (tmp_path / "docs" / "guides" / "guide.md").write_text(
        "# Guide\n\n## Using it\n\n```rust\nlet answer = 42;\n```\n",
        encoding="utf-8",
    )
    return tmp_path


def test_slug_matches_github_heading_anchors():
    assert slug("Supported platforms and feature combinations") == (
        "supported-platforms-and-feature-combinations"
    )
    assert slug("`clifacet::example` is gone") == "clifacetexample-is-gone"
    assert slug("The four layers") == "the-four-layers"


def test_headings_are_collected_from_every_level(docs):
    assert headings((docs / "docs" / "guides" / "guide.md").read_text()) == {"guide", "using-it"}


def test_links_are_read_with_the_line_they_appear_on():
    page = "a\n\n[one](one.md) and [two](two.md)\n"

    assert links(page) == [(3, "one.md"), (3, "two.md")]


def test_an_external_link_is_not_a_file_to_check():
    assert links("[site](https://example.com) [mail](mailto:a@b.c)") == []


def test_a_resolving_tree_has_no_findings(docs):
    assert check(docs) == []


def test_a_link_to_a_missing_file_is_reported(docs):
    page = docs / "README.md"
    page.write_text("# Project\n\nSee the [missing](docs/nope.md).\n", encoding="utf-8")

    findings = check(docs)

    assert [finding.kind for finding in findings] == ["link"]
    assert "docs/nope.md" in findings[0].detail


def test_a_link_to_a_missing_heading_is_reported(docs):
    page = docs / "README.md"
    page.write_text("# Project\n\nSee the [guide](docs/guides/guide.md#nowhere).\n", encoding="utf-8")

    findings = check(docs)

    assert [finding.kind for finding in findings] == ["anchor"]
    assert "nowhere" in findings[0].detail


def test_a_link_to_a_heading_that_exists_is_accepted(docs):
    page = docs / "README.md"
    page.write_text("# Project\n\nSee the [guide](docs/guides/guide.md#guide).\n", encoding="utf-8")

    assert check(docs) == []


def test_rust_blocks_are_collected_and_ignore_blocks_are_skipped(docs):
    page = (docs / "docs" / "guides" / "guide.md").read_text()
    page += "\n```rust,ignore\nthis is not compiled\n```\n\n```sh\ncargo test\n```\n"

    blocks = rust_blocks(page)

    assert [code.strip() for _, code in blocks] == ["let answer = 42;"]


def test_a_snippet_becomes_a_compiled_test(docs):
    generated = generated_snippets(docs)

    assert "#[test]" in generated
    assert "let answer = 42;" in generated
    assert "GENERATED" in generated, "the file says it must not be edited by hand"


def test_hidden_lines_are_unhidden_like_rustdoc_does(docs):
    page = (docs / "docs" / "guides" / "guide.md").read_text()
    page = page.replace("```rust\n", "```rust\n# let hidden = 1;\n")

    generated = generated_snippets(_write(docs, page))

    assert "let hidden = 1;" in generated
    assert "# let hidden" not in generated


def _write(root: Path, page: str) -> Path:
    (root / "docs" / "guides").mkdir(parents=True, exist_ok=True)
    (root / "docs" / "guides" / "guide.md").write_text(page, encoding="utf-8")
    return root


def test_a_snippet_naming_a_symbol_that_does_not_exist_is_reported(docs):
    _write(docs, "# Guide\n\n```rust\nclifacet::policy::NoSuchThing::new();\n```\n")

    findings = [finding for finding in check(docs) if finding.kind == "api"]

    assert findings, "a documented path must resolve to something that exists"
    assert "NoSuchThing" in findings[0].detail


def test_api_paths_are_collected_from_prose_and_code():
    page = "Use `clifacet::policy::Policy::up_to` and `clifacet_example::main`.\n"

    assert api_paths(page) == [
        ("clifacet::policy::Policy::up_to", ("policy", "Policy", "up_to")),
        ("clifacet_example::main", ("main",)),
    ]


def test_an_invented_module_is_reported_as_well_as_an_invented_item():
    """Every segment is checked, not just the last one."""
    page = "# Guide\n\n`clifacet::nonsense::Thing::new()`\n"

    findings = [finding for finding in check(_tree_with(page)) if finding.kind == "api"]

    assert findings and "nonsense" in findings[0].detail


def test_a_domain_word_in_the_reference_example_is_reported():
    """The example is domain-neutral; a guide must not dress it up as a product."""
    page = "# Guide\n\nStore the invoice in the user's todo list.\n"

    findings = [finding for finding in check(_tree_with(page)) if finding.kind == "domain"]

    assert findings, "the reference example must stay domain-neutral"


def _tree_with(page: str) -> Path:
    import tempfile

    root = Path(tempfile.mkdtemp())
    (root / "docs" / "guides").mkdir(parents=True)
    (root / "docs" / "guides" / "guide.md").write_text(page, encoding="utf-8")
    (root / "README.md").write_text("# Project\n", encoding="utf-8")
    return root


def test_a_neutral_guide_has_no_domain_findings():
    page = "# Guide\n\nGreet one person, once per repeat count.\n"

    assert [f for f in check(_tree_with(page)) if f.kind == "domain"] == []


def test_the_checked_pages_are_the_readme_and_the_guides():
    findings = check(ROOT)

    assert findings == [], "the committed documentation tree must be internally consistent"


def test_the_generated_file_is_up_to_date():
    """CI can fail on a stale snippet file, which is what makes the check bite."""
    committed = (ROOT / "examples" / "simple" / "tests" / "documented_snippets.rs").read_text(
        encoding="utf-8"
    )

    assert committed == generated_snippets(ROOT), (
        "run `python scripts/check_docs.py --write` and commit the result"
    )
    assert check_staleness(ROOT) == []


def test_a_finding_renders_the_page_the_line_and_the_problem():
    finding = Finding(kind="link", page=Path("docs/guide.md"), line=7, detail="missing target")

    assert "docs/guide.md:7" in str(finding)
    assert "missing target" in str(finding)


def test_every_section_is_a_directory_the_tree_actually_has():
    """The five sections are a promise about where a page goes; an empty one is a dead end."""
    documented = ROOT / "docs"

    assert {name for name in SECTIONS if (documented / name).is_dir()} == set(SECTIONS)


def test_a_page_beside_the_index_is_reported(tmp_path):
    """`docs/README.md` is the only page at the top level; a second one has no section."""
    (tmp_path / "docs").mkdir()
    (tmp_path / "docs" / "README.md").write_text("# Index\n", encoding="utf-8")
    (tmp_path / "docs" / "stray.md").write_text("# Stray\n", encoding="utf-8")

    findings = uncategorised(tmp_path)

    assert [finding.kind for finding in findings] == ["section"]
    assert findings[0].page.name == "stray.md"


def test_a_page_in_an_invented_directory_is_reported(tmp_path):
    """A sixth directory is a decision, and this is the check that makes it one."""
    (tmp_path / "docs" / "guides").mkdir(parents=True)
    (tmp_path / "docs" / "guides" / "ok.md").write_text("# Fine\n", encoding="utf-8")
    (tmp_path / "docs" / "notes").mkdir()
    (tmp_path / "docs" / "notes" / "loose.md").write_text("# Loose\n", encoding="utf-8")

    findings = uncategorised(tmp_path)

    assert len(findings) == 1
    assert "notes" in findings[0].detail


def test_a_page_in_a_section_is_accepted(tmp_path):
    """Every section must be allowed, not only the ones that happen to have a page today."""
    for name in SECTIONS:
        (tmp_path / "docs" / name).mkdir(parents=True)
        (tmp_path / "docs" / name / "page.md").write_text("# Page\n", encoding="utf-8")

    assert uncategorised(tmp_path) == []
