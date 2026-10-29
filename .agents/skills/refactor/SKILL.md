---
name: refactor
description: Improve structure without changing documented external behavior.
---

# Refactor

Establish a behavior baseline at public seams, identify one structural change,
then make it in small steps. Keep the UI-free command core, shared validation
and explicit registration boundaries. Run the affected Pixi tests and checks;
report any that were not run. Preserve user changes and avoid unrelated edits.
