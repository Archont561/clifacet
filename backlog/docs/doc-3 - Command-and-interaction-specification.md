---
id: doc-3
title: Command and interaction specification
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 15:28'
---
# Status

This document is a proposed implementation target; decision-1 is the accepted architecture and decisions 2–3 carry proposed details. APIs shown are descriptive, not working code.

# Flow

Application registers a clap-derived command and framework metadata explicitly. The framework collects a draft from flags and the selected frontend, validates the completed draft via the original clap command or an explicit compatible typed bridge, then invokes the UI-free handler. Presentation renders the result using the application’s stream and exit mapping.

# Policy

Resolve explicit mode requests before app policy and terminal capability. Record where inputs came from without storing secret values. Non-interactive paths cannot construct a prompter. `--yes` concerns confirmations only. `--json` should imply machine-safe, non-interactive output; finalize flags and precedence with tests.

# Boundaries and open questions

- Canonical argv replay is only safe for supported arguments and must not put secret values into previews or logs. Some parser and clap semantics may require explicit bridges or reject unsupported input.
- Missing input, optional absence and invalid input are distinct states.
- The app owns final exit codes; cancellation and validation failure remain distinguishable.
- The public API, detailed TUI screen contract, behavior for unsupported clap constructs, and handling of secret sources remain to be designed and verified.
