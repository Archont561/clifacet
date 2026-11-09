---
id: CLI-51
title: 'Layer settings from a config file, the environment and flags'
status: Done
assignee: []
created_date: '2026-11-08 02:00'
updated_date: '2026-11-09 11:18'
labels: []
milestone: m-13
dependencies:
  - CLI-46
type: task
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision-19 item 5, which lifts decision-18's "no configuration-file layering yet". File values are injected into clap as defaults, so clap's own order gives flag > environment > file > built-in default and clap's own value parsers validate them. Compiled in the review against clap 4.6.7, with three quirks the library absorbs: clap checks `required` before defaults apply, `ValueParser::parse_ref` is private (a one-argument scratch command runs the real parser), and a default must be `'static` without clap's `string` feature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Precedence is flag > environment > file > built-in default; a file value is validated by its argument's own parser, an error names the file, line and key, and a bad file value is an error even when a flag overrides it
- [x] #2 `--config PATH` before or after the subcommand, `<APP>_CONFIG` and a default location under the platform config directory choose the file, and `--no-config` skips it; a named file that is missing is an error, a missing default file is not
- [x] #3 Keys mirror the flags — a section is a subcommand path and a key a long flag name — and an unknown key or command is an error
- [x] #4 The core takes format-agnostic entries, and an optional feature ships a TOML loader whose dependency is recorded in the audit
- [x] #5 Provenance gains `Source::File`, the record keeps the path and line beside it, and a secret from a file is recorded `file (secret)` without its value
- [x] #6 The environment is read through an injectable source, so the tests never mutate the process environment, and help still names each variable
- [x] #7 A required argument supplied by the file is accepted, and `--help` documents the built-in defaults rather than one machine's file
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented on branch `arena/54e1028a-clifacet`. Decision-19 item 5, taking the shape the review measured: file values injected as clap defaults, so clap's own precedence and value parsers apply rather than a second resolver.

**What exists.** `clifacet::settings` is the core: `Entry` (a section, a key, a value, an optional location), `Settings` (one entry per key, refusing a duplicate), `Settings::apply` (inject, descend, report what was injected), and the `Loader` / `NamedLoader` / `Environment` seams. `settings::source` chooses the file — `--config`, `<APP>_CONFIG`, the platform config directory, `--no-config` — and is the only place that reads one. `settings::toml::Toml` behind `settings-toml` is the loader; `toml_edit` rather than `toml`, because it keeps each value's byte span and a wrong setting therefore names the line it was written on. `App::settings(app, loader)` is the runner's one line, and `Source::File` is the provenance's new variant.

**Three clap facts the review recorded, and what each cost here.**
- `required` is checked against *explicit* presence, so an injected default does not satisfy it. `apply` lifts `required` on an argument a file supplies, which is AC#7's first half.
- `ValueParser::parse_ref` is private, so the file's value is parsed by giving a one-argument scratch command the value through its own long flag. That is why a bad value reports clap's own message verbatim.
- A default must be `'static` without clap's `string` feature. Rather than leak one value per run, the workspace enables `string`, and a test runs a hundred applications to show the value is owned.
- A fourth, found here: `--help` must not read the file. It is answered from the command as built, before the settings path, so help documents the built-in default. The test that caught it showed `[default: 8080]` in help; that is AC#7's second half and it failed before the fix.

**Two bugs the tests found in my own code.** The TOML loader reported its placeholder name (`<toml>`) instead of the file, so a real error said `<toml>:2`; `NamedLoader` exists because of that. And `--config` was only declared when the application flattened `Globals`, so an application with its own root got `unexpected argument '--config'`; the settings flags are now declared by the settings feature, at the command `App::settings` builds.

**Two judgement calls.** A bool flag is set by `true` and left alone by `false`, and anything else is refused — `--quiet = "yes"` meant one of two things and guessing which is worse than asking. And a key set twice is refused rather than last-one-wins, because a file that says two things about one setting has a mistake in it that is easier to fix where it is.

**Verification.** 68 test binaries pass with `--all-features`; `settings` (25), `settings_toml` (11) and `settings_run` (14) are new. Every feature row compiles, including `no-default-features` and `settings-toml` alone; `deny` reports licenses ok with `toml_edit` under MIT OR Apache-2.0, which is AC#4's audit requirement; clippy, fmt, alint, check-docs, prose (Vale), skills-check, backlog-check and changelog-check are clean.
<!-- SECTION:NOTES:END -->
