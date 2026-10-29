---
name: audit
description: Review security, dependencies, generated workflows, trust boundaries, or release changes.
---

# Audit

Identify the trust boundary, external inputs and observable failure modes.
Inspect the changed source and relevant decisions; check secret handling,
permissions, network/offline behavior, dependency advisories and recovery.
Run `pixi run --frozen deny` and `pixi run --frozen audit` when feasible.
Distinguish verified findings from hypotheses, cite evidence, and disclose the
existing narrow `RUSTSEC-2026-0374` exception. Do not add silent suppressions.
