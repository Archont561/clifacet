---
id: doc-6
title: Agent operating model and direction reset
type: other
created_date: '2026-10-29 15:28'
updated_date: '2026-10-29 15:28'
---
# Skills

Adopt a single deliberate skills source/lock model; existing Archont561 repos use different models. Candidate skills: session, backlog, grill-me, tdd, bdd, refactor, audit, clean-code-principles, writing-for-agents. Use only reviewed, actually installed skill content, and validate committed skills in CI.

# Local regain-direction skill

When the maintainer feels lost or a session drifts: pause implementation; inspect current context, working tree, active task and available test results; summarize goal, verified work, open items, drift and unknowns; suggest up to three concrete next actions, including one small low-risk action; recommend one and ask the maintainer if a choice is needed.

Do not discard changes, rewrite plans, close tasks, commit, push or release without authorization. Resume within the selected scope. The skill complements the session skill and is development guidance, not a published Rust dependency.
