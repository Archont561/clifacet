---
id: decision-20
title: Vale checks the prose, alint checks the repository, and one gate runs both
date: '2026-11-08 17:30'
status: accepted
---
## Context

`alint check` already holds this repository together: it proves the files a contributor
needs exist, that the workspace membership is what a reviewer agreed to, that every
workflow is pinned, and that the generated sandbox files match their policy. What it
cannot do is read a sentence. A rule that says "do not write 'behaviour'" is not a
file-existence question, and a check that cannot see a paragraph cannot answer it.

The maintainer asked for Vale to be adopted as a second, independent layer: Vale for
prose (spelling, terminology, editorial style), alint for repository structure, and
neither moved into the other.

## Decision

1. **Vale, and only for prose.** `.vale.ini` names the files it lint. That is `README.md`,
   `AGENTS.md` and `docs/**/*.md` — the pages a reader is meant to read. `backlog/` is
   tool-shaped and `CHANGELOG.md` is generated; both already have a check (`backlog-check`,
   `changelog-check`), and both would produce findings a writer cannot fix.

2. **Two pinned style packages, never names.** `Microsoft` and `write-good` are installed
   from release URLs, so an upstream update cannot change what CI rejects without a commit
   here. A style installed by name takes whatever is newest.

3. **`vale.toml` is authored; `.vale.ini` and `.vale/styles/` are generated.** Vale reads
   an INI file and a directory of YAML rules: both are configuration, and hand-edited
   configuration is where a rule with no stated reason comes from. `scripts/prose.py`
   renders them, and `prose-check` fails when the committed copy has drifted — the same
   arrangement `scripts/restore.sh` and the documented snippets already use. Each rule the
   repository switches off carries its reason in `vale.toml`, and the check refuses a `NO`
   with no reason; the reason reaches the INI as a comment beside the switch.

4. **The repository's own style is one rule and one vocabulary.** `Clifacet.American` is a
   substitution rule for the spellings this prose actually uses (`behaviour`,
   `normalise`/`serialise`/`materialise`/`initialise`, `labelled`, `cancelled`). The
   vocabulary lists the project's own words (`clifacet`, `pixi`, `alint`, `clap`,
   `ratatui`, …) so both spelling rules can read them. `Vale.Terms` is off: tool names
   appear both as prose and as command text, and a case rule cannot tell `pixi run` from a
   sentence about Pixi.

5. **House style is declared, not inherited silently.** Ten inherited rules are switched
   off with a reason, because this prose is written that way deliberately: spaced em dashes,
   `cannot` over `can't`, paired clauses joined by a semicolon, no author voice, `backend`
   as the domain's word, and no E-Prime. A new exception has to say why.

6. **One gate.** `pixi run prose-check` renders-and-verifies and then lints; it is in
   `test-local` and in `quality`, so a contributor's commit gate and CI run the same check.
   It is offline-safe because the packages are committed after `prose-sync`, which is the
   only fetching task.

## Consequences

- **Vale joins the environment**, pinned `>=3.26.0,<3.27` from conda-forge (MIT). The
  sandbox restores it like any other dependency, and the Pixi lock changes with it.
- **Editors and agents need to know which tool to run.** `AGENTS.md` gains a section naming
  the two commands and when each applies; `docs/development.md` explains the split and how
  to change a rule.
- **A rule is either a Vale rule or an alint rule, never both.** A repository convention
  stays in `.alint.yml`; a wording preference stays in `vale.toml`.
- **The prose now has to pass.** The first run fixes what the check found: five `behaviour`
  among nine `behavior`, four `cancelled` runs beside the `Failure::cancelled()` API name,
  and the `-ise` spellings. Code and API names are untouched — Vale reads prose, and code
  spans are ignored by default.
- **This is not a spell-checker for code.** A `rust` fence or an inline `code` span is
  skipped, so an identifier is never "misspelled" and the guides may quote the API exactly.
