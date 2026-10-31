#!/usr/bin/env python3
"""Check the guides: links resolve, snippets compile, and the API is real.

Three claims the documentation makes about itself, none of which survive being
maintained by hand for long:

- **A link goes somewhere.** Every relative link resolves to a file that exists,
  and every `#fragment` to a heading that is in that file.
- **A snippet is the code that exists.** Rust fences use rustdoc's own convention:
  `rust` is compiled, `rust,ignore` is an illustration that is not. The compiled
  ones are written into a generated test file, so a snippet cannot rot into
  something that only looks like Rust — and because the file is committed, CI
  fails when it is stale rather than silently regenerating it.
- **The names are the crate's.** Every `clifacet::…` or `clifacet_example::…` path
  in prose or code has to end in a symbol the crate defines. A guide that invents
  a function is a guide that will teach someone a function that does not exist.

A fourth, smaller rule: the reference application is domain-neutral on purpose, so
a page must not dress it up as a product. The word list is deliberately short and
concrete; it names the domains the example is *not*.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterator, Sequence
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
README = ROOT / "README.md"
DOCS = ROOT / "docs"
SNIPPETS = ROOT / "crates" / "clifacet-example" / "tests" / "documented_snippets.rs"

# The crates a guide may name. Anything else is not checked, so a guide may still
# mention `std`, `ratatui` or `clap` without this script pretending to know them.
CRATES = ("clifacet", "clifacet_example")
SOURCE_ROOTS = (
    ROOT / "crates" / "clifacet" / "src",
    ROOT / "crates" / "clifacet-example" / "src",
)

# Domains the reference application deliberately is not. Kept short and concrete:
# this is a guard against a guide quietly growing a second, imaginary product.
# Not "checkout": that is also what Git does, and `development.md` means the Git one.
DOMAIN_WORDS = ("invoice", "todo list", "shopping cart", "customer record", "order number")

LINK = re.compile(r"\[[^\]]+\]\(([^)\s]+)\)")
HEADING = re.compile(r"^(#{1,6})\s+(.*)$", re.MULTILINE)
FENCE = re.compile(r"^```(\S*)\s*$", re.MULTILINE)
API_PATH = re.compile(r"\b((?:%s)(?:::[A-Za-z_][A-Za-z0-9_]*)+)" % "|".join(CRATES))
# `pub fn x`, `pub struct X`, `pub(crate) fn x` (not public, so not listed), `pub use a::{b, c}`.
DECLARATION = re.compile(
    r"^\s*pub\s+(?!\(crate\))(?:fn|struct|enum|trait|const|static|type|mod|use)\s+(.+)$",
    re.MULTILINE,
)


@dataclass(frozen=True)
class Finding:
    """One documentation problem, with the page and line that caused it."""

    kind: str
    page: Path
    line: int
    detail: str

    def __str__(self) -> str:
        return f"{self.page.relative_to(ROOT) if self.page.is_relative_to(ROOT) else self.page}:{self.line}: {self.kind}: {self.detail}"


def pages(root: Path = ROOT) -> list[Path]:
    """Every page this script checks: the README and the guides beside it."""
    readme = root / "README.md"
    guides = sorted((root / "docs").glob("*.md"))
    return ([readme] if readme.is_file() else []) + guides


def slug(heading: str) -> str:
    """GitHub's heading anchor: lower case, punctuation dropped, spaces to dashes.

    Backticks and other markup are removed rather than transliterated, which is why
    `` `clifacet::example` is gone `` anchors as `clifacetexample-is-gone`.
    """
    text = heading.strip().lower()
    text = re.sub(r"[`*_]", "", text)
    text = re.sub(r"[^a-z0-9 \-_]", "", text)
    return text.strip().replace(" ", "-")


def headings(text: str) -> set[str]:
    """Every anchor a heading in `text` provides."""
    return {slug(match.group(2)) for match in HEADING.finditer(text)}


def links(text: str) -> list[tuple[int, str]]:
    """Relative link targets with the line they appear on; external links are skipped."""
    found = []
    for number, line in enumerate(text.splitlines(), start=1):
        for target in LINK.findall(line):
            if target.startswith(("http://", "https://", "mailto:", "#")) is False:
                found.append((number, target))
    return found


def check_links(root: Path = ROOT) -> list[Finding]:
    """Every relative link resolves to a file, and any fragment to a heading in it."""
    findings = []
    for page in pages(root):
        for line, target in links(page.read_text(encoding="utf-8")):
            path, _, fragment = target.partition("#")
            destination = (page.parent / path).resolve() if path else page
            if not destination.exists():
                findings.append(Finding("link", page, line, f"{target} does not resolve"))
                continue
            if fragment and destination.suffix == ".md":
                anchors = headings(destination.read_text(encoding="utf-8"))
                if fragment not in anchors:
                    findings.append(
                        Finding("anchor", page, line, f"{target}: no heading anchors to '{fragment}'")
                    )
    return findings


def rust_blocks(text: str) -> list[tuple[int, str]]:
    """The Rust fences that must compile, as `(line, code)`.

    Fences tagged `ignore` are illustrations — rustdoc would not compile them either —
    and fences in any other language are not code at all.
    """
    blocks = []
    lines = text.splitlines()
    index = 0
    while index < len(lines):
        match = FENCE.match(lines[index])
        if match is None:
            index += 1
            continue
        info = match.group(1)
        start = index + 1
        body = []
        index += 1
        while index < len(lines) and not lines[index].startswith("```"):
            body.append(lines[index])
            index += 1
        index += 1
        tags = [tag.strip() for tag in info.split(",")] if info else []
        if tags[:1] == ["rust"] and "ignore" not in tags:
            blocks.append((start, "\n".join(body)))
    return blocks


def _snippet_body(code: str) -> str:
    """The code a test body needs: rustdoc's hidden `#` lines unhidden."""
    lines = [line[2:] if line.startswith("# ") else line for line in code.splitlines()]
    return "\n".join(line for line in lines if line.strip()).strip()


def generated_snippets(root: Path = ROOT) -> str:
    """The test file that makes every compiled snippet part of the suite."""
    entries = []
    for page in pages(root):
        name = page.stem.lower().replace("-", "_")
        for number, (line, code) in enumerate(rust_blocks(page.read_text(encoding="utf-8")), start=1):
            body = _snippet_body(code)
            if not body:
                continue
            entries.append(
                f"/// `{page.name}`, line {line}.\n"
                # The body is a verbatim copy of the page, so rustfmt must not rewrite it:
                # `cargo fmt --all` and `--write` would otherwise disagree forever.
                f"#[rustfmt::skip]\n"
                f"#[test]\n"
                f"fn {name}_snippet_{number}() {{\n"
                f"    #![allow(unused_variables, unused_imports, dead_code, path_statements)]\n"
                + "".join(f"    {snippet_line}\n" for snippet_line in body.splitlines())
                + "}\n"
            )
    loader = SNIPPETS.name
    header = (
        "//! Every Rust snippet the guides publish, compiled as part of the suite.\n"
        "//!\n"
        "//! GENERATED by `scripts/check_docs.py` — do not edit. A fence tagged `rust` in\n"
        "//! `README.md` or `docs/*.md` becomes a test here, so a snippet cannot drift into\n"
        "//! something that only looks like Rust. `rust,ignore` fences are illustrations and\n"
        "//! are not copied. Run `python scripts/check_docs.py --write` after editing a page.\n\n"
        "#![allow(\n"
        "    clippy::items_after_statements,\n"
        "    clippy::needless_pass_by_value,\n"
        "    clippy::no_effect_underscore_binding,\n"
        "    clippy::unnecessary_wraps,\n"
        "    reason = \"a snippet is a verbatim copy of a published guide, so its shape is \\\n"
        "              documentation; what this file proves is that the code compiles and runs\"\n"
        ")]\n\n"
        f"// The name of this file is {loader}; it is the tests/ target of the example crate.\n\n"
    )
    return header + "\n".join(entries)


def reexported_modules(root: Path = ROOT) -> set[str]:
    """The names of third-party items a crate re-exports whole.

    `pub use ratatui;` makes `clifacet::tui::ratatui::layout::Rect` a path into
    *ratatui*, not into this crate, so checking its segments against this crate's
    declarations would be wrong. A re-export of another crate is a boundary.
    """
    modules: set[str] = set()
    for source_root in SOURCE_ROOTS:
        for path in source_root.rglob("*.rs"):
            for match in re.finditer(r"^\s*pub\s+use\s+([A-Za-z_][A-Za-z0-9_]*)\s*;", path.read_text(encoding="utf-8"), re.MULTILINE):
                if match.group(1) not in {"crate", "self", "super"}:
                    modules.add(match.group(1))
    return modules


def declared_symbols(root: Path = ROOT) -> set[str]:
    """Every name a crate defines publicly, including its re-exports."""
    symbols: set[str] = set()
    for source_root in SOURCE_ROOTS:
        for path in source_root.rglob("*.rs"):
            for declaration in DECLARATION.findall(path.read_text(encoding="utf-8")):
                # `foo`, `foo<T>`, `foo::{a, b}`, `foo as bar` — the last segment is a name.
                tail = declaration.split("::")[-1] if "{" not in declaration else ""
                for candidate in re.findall(r"[A-Za-z_][A-Za-z0-9_]*", declaration):
                    symbols.add(candidate)
                if tail:
                    symbols.add(tail.strip(" ;{}"))
    return symbols


def api_paths(text: str) -> list[tuple[str, tuple[str, ...]]]:
    """Every documented path of a checked crate, with the segments it claims exist."""
    found = []
    for match in API_PATH.finditer(text):
        path = match.group(1)
        segments = tuple(segment for segment in path.split("::")[1:] if segment)
        found.append((path, segments))
    return found


def check_api(root: Path = ROOT) -> list[Finding]:
    """Every segment of a documented path is a public module or a public item.

    Checking the last segment alone would let `clifacet::policy::NoSuchThing::new()`
    pass on the strength of `new` existing somewhere else entirely, which is exactly
    the kind of invented API this is meant to catch.
    """
    symbols = declared_symbols(root)
    boundaries = reexported_modules(root)
    findings = []
    for page in pages(root):
        text = page.read_text(encoding="utf-8")
        for number, line in enumerate(text.splitlines(), start=1):
            for path, segments in api_paths(line):
                # Everything after a re-exported module belongs to that other crate.
                checked = []
                for segment in segments:
                    if segment in boundaries:
                        break
                    checked.append(segment)
                missing = [segment for segment in checked if segment not in symbols]
                if missing:
                    findings.append(
                        Finding(
                            "api",
                            page,
                            number,
                            f"`{path}`: no public item named `{missing[0]}`",
                        )
                    )
    return findings


def check_domain(root: Path = ROOT) -> list[Finding]:
    """The reference application stays domain-neutral."""
    findings = []
    for page in pages(root):
        for number, line in enumerate(page.read_text(encoding="utf-8").splitlines(), start=1):
            lowered = line.lower()
            for word in DOMAIN_WORDS:
                if word in lowered:
                    findings.append(
                        Finding(
                            "domain",
                            page,
                            number,
                            f"'{word}' dresses the neutral reference example as a product",
                        )
                    )
    return findings


def check_staleness(root: Path = ROOT) -> list[Finding]:
    """The committed snippet file is what the pages currently generate."""
    target = root / "crates" / "clifacet-example" / "tests" / "documented_snippets.rs"
    if not target.is_file():
        return [Finding("snippets", target, 1, "the generated snippet file is missing")]
    if target.read_text(encoding="utf-8") != generated_snippets(root):
        return [
            Finding(
                "snippets",
                target,
                1,
                "out of date; run `python scripts/check_docs.py --write` and commit it",
            )
        ]
    return []


def check(root: Path = ROOT) -> list[Finding]:
    """Every finding about the pages themselves, in page order.

    `check_staleness` is deliberately separate: it is a question about the generated
    file, not about a page, and a caller checking a tree of pages should not have to
    create that file to do it.
    """
    return check_links(root) + check_api(root) + check_domain(root)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="regenerate the snippet test file")
    mode.add_argument("--check", action="store_true", help="verify the committed tree (the default)")
    arguments = parser.parse_args(argv)

    if arguments.write:
        SNIPPETS.write_text(generated_snippets(ROOT), encoding="utf-8")
        print(f"check-docs: wrote {SNIPPETS.relative_to(ROOT)}")
        return 0

    findings = check(ROOT) + check_staleness(ROOT)
    if findings:
        for finding in findings:
            print(f"check-docs: {finding}", file=sys.stderr)
        return 1
    print(
        f"check-docs: {len(pages(ROOT))} page(s), every link resolves, every snippet is "
        "compiled, every documented name exists"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
