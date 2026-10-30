---
id: decision-12
title: Redaction by type, and provenance without values
date: '2026-10-31 12:00'
status: accepted
---
## Context
A command-line application routinely accepts credentials, and a framework that formats values for diagnostics, previews and test records is the natural place for them to leak. The maintainer asked for framework-owned secret and provenance guarantees: redaction in everything the framework shows, provenance that records where a value came from but not the value, and an honest list of what the framework cannot protect.

## Decision
Carry secrets in a `Secret<T>` newtype whose `Debug` and `Display` are a fixed `[redacted]` marker, and parse it through `FromStr` so `clap` accepts it unchanged. Do redaction by type rather than by scrubbing output: every framework-owned surface already formats values through `Debug` or `Display`, so a value that cannot print itself cannot leak through them, and no list of secret values has to be kept. Record provenance in a `Provenance` whose entries hold a name, a `Source` and whether the value is secret — there is no field for a value, so the record cannot leak one. Do not add scrubbing of app-built strings, secret-aware prompt adapters, or automatic provenance: only the application knows where a value came from and what it wrote into its own log.

## Consequences
Applications mark their own secrets by using `Secret<T>`, and a value that stays a plain `String` is not protected — the guarantee is about the framework's types, not about a policy the framework can enforce. `Secret::expose` is the only read, so every place that touches a credential is visible in review. Masking in a form is presentation only; the submission still carries the value. Secrets supplied on the command line are visible to other processes on most systems, a prompted secret is echoed by the terminal, and application-owned logs are the application's responsibility; all three are documented as limitations rather than papered over. Tests assert the absence of a canary without printing it, so a failing redaction test does not leak the value it is protecting.
