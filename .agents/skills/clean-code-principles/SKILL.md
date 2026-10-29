---
name: clean-code-principles
description: Apply focused clean-code principles when designing or reviewing implementation changes.
---

# Clean code

Favor clear names, small cohesive modules, explicit errors and one source of
truth. Separate policy from adapters and presentation from execution. Extract
abstractions when real duplication or a stable seam justifies them; do not add
speculative plugin systems, global registration or generic widget backends.
Keep changes proportionate and make behavior observable through public tests.
