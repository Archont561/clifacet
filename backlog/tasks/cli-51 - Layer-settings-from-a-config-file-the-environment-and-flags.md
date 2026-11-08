---
id: CLI-51
title: 'Layer settings from a config file, the environment and flags'
status: To Do
assignee: []
created_date: '2026-11-08 02:00'
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
- [ ] #1 Precedence is flag > environment > file > built-in default; a file value is validated by its argument's own parser, an error names the file, line and key, and a bad file value is an error even when a flag overrides it
- [ ] #2 `--config PATH` before or after the subcommand, `<APP>_CONFIG` and a default location under the platform config directory choose the file, and `--no-config` skips it; a named file that is missing is an error, a missing default file is not
- [ ] #3 Keys mirror the flags — a section is a subcommand path and a key a long flag name — and an unknown key or command is an error
- [ ] #4 The core takes format-agnostic entries, and an optional feature ships a TOML loader whose dependency is recorded in the audit
- [ ] #5 Provenance gains `Source::File`, the record keeps the path and line beside it, and a secret from a file is recorded `file (secret)` without its value
- [ ] #6 The environment is read through an injectable source, so the tests never mutate the process environment, and help still names each variable
- [ ] #7 A required argument supplied by the file is accepted, and `--help` documents the built-in defaults rather than one machine's file
<!-- AC:END -->
