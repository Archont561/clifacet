---
id: m-7
title: "M7 — Declarative command surface"
---

## Description

The input side of a command as declarations instead of builder code: field syntax, a nested tree, one file per command, one macro-layering rule, and an input type that does not have to be a literal struct path.

Exit: a command in the reference application is declared as a struct with attributed fields plus a doc comment, its nesting is declared rather than assembled, registration is one file per command, and decisions 13 to 16 are shipped with trybuild coverage of what must not compile.
