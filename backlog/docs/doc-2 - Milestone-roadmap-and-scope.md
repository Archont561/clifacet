---
id: doc-2
title: Milestone roadmap and scope
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-11-03 07:49'
---
# Roadmap

The canonical milestone files describe exits; tasks are assigned to their relevant milestone. All items are unimplemented unless a task and evidence say otherwise.

1. **m-0 Foundation:** environment, repository gates, sandbox and release-workflow feasibility.
2. **m-1 Clap-first core:** one domain-neutral, non-interactive vertical slice.
3. **m-2 Interactive:** policy and prompts with no forbidden prompter construction.
4. **m-3 TUI:** public ratatui components, generated form and overrides.
5. **m-4 Parity and safety:** public cross-mode harness, output and secret guarantees.
6. **m-5 0.1.0:** qualification, real plain docs and authorized release readiness.
7. **m-6 Authoring ergonomics:** choice, secret and repeated questions, a classified confirmation, provenance from the parse, a plan seam, parity sugar and one failure renderer — decision-18, taken from the backlog.md wrapper.
8. **m-7 Declarative command surface:** an argument declared once as an attributed struct field, a nested tree, one command per file, one macro-layering rule, and an input type that need not be a literal struct path — decisions 13 to 16.
9. **m-8 Plugins as binaries on the path:** a `<cli>-<name>` binary exposed as `<cli> <name>` with verbatim argv forwarding and exit-status passthrough, on Unix and Windows — decision-17.
10. **m-9 Feature boundaries and machine-readable output:** every declared feature proved to have a `cfg` site, `serde` made optional behind a feature that serialises failures with secrets redacted, and the `json` feature either implemented on that evidence or retired.
11. **m-10 Release 0.2.0:** the version-bump runbook exercised end to end on a real bump, every 0.1.0 gate passing again, and both crates published from one authorized dispatch.

Each stage includes its own behavior and safety tests; m-4 is completion and publication of the shared harness, not permission to defer testing. Decision-13 (one declarative attribute per field) was accepted on 2026-11-03 and built on CLI-28, now Done; the rest of m-6 to m-10 rests on **proposed** decisions (14 to 18) that a maintainer has not yet accepted, so those task lists are designs rather than authorized work. Milestones are planning boundaries, not guaranteed schedules. Plan/dry-run and events were outside the default 0.1.0 scope; a plan seam is now m-6 work and events remain out of scope.
