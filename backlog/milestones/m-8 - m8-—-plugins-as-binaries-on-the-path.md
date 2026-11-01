---
id: m-8
title: "M8 — Plugins as binaries on the path"
---

## Description

A <cli>-<name> binary on the path exposed as <cli> <name>, the way git-lfs becomes git lfs. Design and the measured clap behaviour are decision-17; the card carries the whole feature, including the Windows gaps the prototype did not cover.

Exit: discovery is explicit and never executes, help lists plugins with their path, built-ins always win with the drop reported, a plugin's argv is forwarded verbatim and its exit status passes through, and the Unix and Windows jobs both exercise it.
