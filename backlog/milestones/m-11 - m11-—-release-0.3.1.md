---
id: m-11
title: "M11 — Release 0.3.1"
---

## Description

A patch release for two findings from the design review after 0.3.0: a secret read from the environment was shown by `--help`, because clap prints a variable's current value beside its name (decision-12), and the library forced clap's colour and suggestion features on every application. It is also the real version bump CLI-20's last criterion asked to rehearse.

Exit: a `secret` field never shows its environment value in help, clifacet depends on clap without its default features, `bump-version 0.3.1 --relock` lands through a PR, and both crates publish from one maintainer dispatch.
