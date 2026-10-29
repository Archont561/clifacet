---
id: doc-2
title: Milestone roadmap and scope
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 15:28'
---
# Roadmap

The canonical milestone files describe exits; tasks are assigned to their relevant milestone. All items are unimplemented unless a task and evidence say otherwise.

1. **m-0 Foundation:** environment, repository gates, sandbox and release-workflow feasibility.
2. **m-1 Clap-first core:** one domain-neutral, non-interactive vertical slice.
3. **m-2 Interactive:** policy and prompts with no forbidden prompter construction.
4. **m-3 TUI:** public ratatui components, generated form and overrides.
5. **m-4 Parity and safety:** public cross-mode harness, output and secret guarantees.
6. **m-5 0.1.0:** qualification, real plain docs and authorized release readiness.

Each stage includes its own behavior and safety tests; m-4 is completion and publication of the shared harness, not permission to defer testing. Milestones are planning boundaries, not guaranteed schedules. Plan/dry-run and events are outside the default 0.1.0 scope; an explicit decision is needed to bring them forward.
