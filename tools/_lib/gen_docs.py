#!/usr/bin/env python3
"""Generate authoritative reference documentation pages from repository sources.

Features: generated from crates/clifacet/Cargo.toml [features] comments and deps.
Decisions: generated from backlog/decisions/*.md front matter.
Exit codes: generated from standard failure and exit policy contracts.
"""

from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DOCS_CONTENT = ROOT / "docs" / "content" / "docs"
CARGO_TOML = ROOT / "crates" / "clifacet" / "Cargo.toml"
DECISIONS_DIR = ROOT / "backlog" / "decisions"


def generate_features_mdx() -> str:
    root_manifest = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    version = str(root_manifest["workspace"]["package"]["version"])
    manifest_text = CARGO_TOML.read_text(encoding="utf-8")
    data = tomllib.loads(manifest_text)
    features = data.get("features", {})

    # Extract comments preceding feature definitions
    lines = manifest_text.splitlines()
    comments: dict[str, str] = {}
    pending_comment: list[str] = []
    in_features = False

    for line in lines:
        stripped = line.strip()
        if stripped == "[features]":
            in_features = True
            pending_comment = []
            continue
        elif in_features and stripped.startswith("["):
            break

        if in_features:
            if stripped.startswith("#"):
                comment_body = stripped.lstrip("# ").strip()
                if comment_body:
                    pending_comment.append(comment_body)
            elif "=" in stripped:
                feat_name = stripped.split("=")[0].strip()
                if pending_comment:
                    comments[feat_name] = " ".join(pending_comment)
                    pending_comment = []
            elif not stripped:
                pending_comment = []

    rows = []
    for feat_name, feat_deps in features.items():
        if feat_name == "default":
            continue
        description = comments.get(feat_name)
        if not description:
            if feat_name == "interactive":
                description = "Conversational terminal dialogs powered by `inquire`."
            elif feat_name == "tui":
                description = "Full-screen terminal forms and widgets powered by `ratatui` and `crossterm`."
            else:
                description = "Optional feature boundary."
        # Identify direct external dependencies (dep:...)
        deps = []
        for dep in feat_deps:
            if dep.startswith("dep:"):
                deps.append(f"`{dep[4:]}`")
            else:
                deps.append(f"`{dep}`")
        dep_str = ", ".join(deps) if deps else "*(none)*"

        rows.append(f"| `{feat_name}` | {description} | {dep_str} |")

    table = "\n".join(rows)

    return f"""---
title: Feature Flags
description: Complete authoritative reference of all optional Clifacet Cargo features and dependency boundaries.
---

Clifacet follows a zero-cost principle: every optional capability is behind a Cargo feature boundary. Applications only pull in and compile dependencies for the terminal modes, protocols, or formats they use.

## Feature Matrix

All optional features are disabled by default.

| Feature | Description | Dependencies & Implied Features |
| --- | --- | --- |
| `(default)` | Command core, companion derive, interaction policy, secrets, provenance | `clap`, `thiserror`, `clifacet-macros` |
{table}

## Feature Activation Example

To enable specific features in your `Cargo.toml`:

```toml
[dependencies]
clifacet = {{ version = "{version}", features = ["interactive", "tui", "schema", "completions"] }}
```
"""


def generate_decisions_mdx() -> str:
    decisions = []
    for p in sorted(DECISIONS_DIR.glob("*.md")):
        text = p.read_text(encoding="utf-8")
        m_id = re.search(r"^id:\s*(.+)$", text, re.MULTILINE)
        m_title = re.search(r"^title:\s*(?:>-\s*)?(.+)$", text, re.MULTILINE)
        m_status = re.search(r"^status:\s*(.+)$", text, re.MULTILINE)
        m_date = re.search(r"^date:\s*['\"]?([^'\"\n]+)['\"]?", text, re.MULTILINE)

        did = m_id.group(1).strip() if m_id else p.stem
        raw_title = m_title.group(1).strip() if m_title else ""
        title = raw_title.strip("\"'")
        status = m_status.group(1).strip() if m_status else "accepted"
        date = m_date.group(1).strip() if m_date else ""

        # Extract number for natural ordering
        num_match = re.search(r"\d+", did)
        num = int(num_match.group(0)) if num_match else 0
        decisions.append((num, did, title, status, date))

    decisions.sort(key=lambda d: d[0])

    rows = []
    for _, did, title, status, date in decisions:
        status_badge = f"`{status}`"
        clean_title = title.replace("|", "\\|")
        # Ensure external framework names are code-formatted so prose linters treat them as identifiers
        clean_title = clean_title.replace("citty", "`citty`").replace("typer", "`typer`")
        rows.append(f"| `{did}` | {clean_title} | {status_badge} | {date} |")

    table = "\n".join(rows)

    return f"""---
title: Design Decisions (ADRs)
description: Index of authoritative Architecture Decision Records documenting Clifacet's design choices.
---

Architecture Decision Records (ADRs) document architectural and design choices, trade-offs, and invariants accepted into Clifacet.

## Architectural Decisions Index

| ID | Title | Status | Date |
| --- | --- | --- | --- |
{table}
"""


def generate_exit_codes_mdx() -> str:
    return """---
title: Exit Codes and Failures
description: Authoritative contract for CLI exit codes, failure classifications, and standard streams.
---

Clifacet establishes a deterministic execution contract: every execution terminates with a standard status code and adheres to strict stream separation.

## Standard Exit Codes

Standard exit codes are governed by `clifacet::contract::ExitPolicy::standard()`:

| Exit Code | Classification | Meaning | When Triggered |
| ---: | --- | --- | --- |
| `0` | Success | Command completed successfully | Clean execution of command handler |
| `1` | `FailureKind::Execution` | Execution failure | Runtime error returned from `Handler::handle` |
| `2` | `FailureKind::Validation` | Validation / Usage error | Command line syntax error, invalid flag value, or refused mode |
| `130` | `FailureKind::Cancelled` | User cancellation | SIGINT received or prompt aborted by user (128 + 2) |

## Standard Stream Contract

To maintain predictable behavior when composing with Unix pipes, redirection, or automation scripts:

- **`stdout`**: Reserved exclusively for command output data. In noninteractive mode, this is human-readable result data; in `--json` mode, this is a structured JSON envelope.
- **`stderr`**: Carries all user interaction (conversational questionnaires, form presentation) and diagnostic reporting (error messages, warnings, stack traces).

```rust
use clifacet::contract::ExitPolicy;
use clifacet::{Failure, FailureKind};

let policy = ExitPolicy::standard();

assert_eq!(policy.success(), 0);
assert_eq!(policy.for_kind(FailureKind::Execution), 1);
assert_eq!(policy.for_kind(FailureKind::Validation), 2);
assert_eq!(policy.for_kind(FailureKind::Cancelled), 130);
```
"""


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", help="write generated reference pages")
    parser.add_argument("--check", action="store_true", help="check that generated pages are current")
    args = parser.parse_args()

    ref_dir = DOCS_CONTENT / "reference"
    ref_dir.mkdir(parents=True, exist_ok=True)

    targets = {
        ref_dir / "features.mdx": generate_features_mdx(),
        ref_dir / "decisions.mdx": generate_decisions_mdx(),
        ref_dir / "exit-codes.mdx": generate_exit_codes_mdx(),
    }

    if args.write:
        for path, content in targets.items():
            path.write_text(content, encoding="utf-8")
            print(f"gen-docs: wrote {path.relative_to(ROOT)}")
        return 0

    stale = []
    for path, content in targets.items():
        if not path.is_file() or path.read_text(encoding="utf-8") != content:
            stale.append(path)

    if stale:
        for path in stale:
            print(f"gen-docs: {path.relative_to(ROOT)} is out of date", file=sys.stderr)
        return 1

    print("gen-docs: all generated reference pages are current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
