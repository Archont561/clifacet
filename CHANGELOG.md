# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
This file is generated: `pixi run changelog` renders it from the conventional
commits with `convco`, and `pixi run changelog-check` fails when the committed
copy has drifted. Edit the commit messages, not the file.

## [Unreleased]

## [0.1.0] - 2026-11-01

### ⚠ BREAKING CHANGE

* `clifacet::example` is removed. Use the `clifacet-example` crate.

### Added

* **prompt:** ship an inquire dialog as the interactive frontend
([a64437c](https://github.com/Archont561/clifacet/commit/a64437c9e40b79a4b3df22319fc97d5aa4f45fdf))
* **policy:** prefer a default mode, and move the example to examples/simple
([f17e64b](https://github.com/Archont561/clifacet/commit/f17e64b32aa6e3d1d99fd2c4f3bf14c4624442a3))
* **macros:** wrap clap's parser and the input bridge in one attribute
([9578daa](https://github.com/Archont561/clifacet/commit/9578daa69bac3525abb6a4b91a5b7d0efc784eed))
* **tooling:** check what a release would publish
([359a203](https://github.com/Archont561/clifacet/commit/359a203c4a429e25935aaa14976e69caffaf3b8b))
* **tooling:** give the advisory exception an expiry
([8200144](https://github.com/Archont561/clifacet/commit/8200144ba4f252d7c97499dc62619a68677c5b62))
* **contract:** make the stream and exit contracts explicit and provable
([3d66018](https://github.com/Archont561/clifacet/commit/3d6601891ce0e0f88d232ceea98a84b6e7e7510f))
* **secret:** redact by type and record provenance without values
([603d756](https://github.com/Archont561/clifacet/commit/603d7569a832553d78b05a91901a6c96af2dda25))
* **harness:** publish a cross-mode parity harness
([dfb51e0](https://github.com/Archont561/clifacet/commit/dfb51e0583e347d834b0d05ceb81e126a6dc7d73))
* **tui:** add custom field and whole-screen escape hatches
([3935ea4](https://github.com/Archont561/clifacet/commit/3935ea498abd3fe9b22ac15f33ef01e6d2c665b4))
* **tui:** ship public ratatui components and a generated form
([7ac545b](https://github.com/Archont561/clifacet/commit/7ac545b4bb1c554a6303aa3b52f5ba7ee7a9cce4))
* **prompt:** collect missing values without re-asking supplied ones
([2f6ffee](https://github.com/Archont561/clifacet/commit/2f6ffee65a55a74de856e90ed01284da105fcca0))
* **policy:** resolve the interaction mode before anything runs
([aef5893](https://github.com/Archont561/clifacet/commit/aef5893e4ff540518dce02e0e50e415421877589))
* **macros:** derive Facet to bridge clap commands to typed input
([8828597](https://github.com/Archont561/clifacet/commit/88285976efc0812708c8a754f1a2f2b96b0da44c))
* **core:** add the UI-free command seam and a reference command
([300b611](https://github.com/Archont561/clifacet/commit/300b611b9d4a04e595d52b5caf1b7ac5bac8f994))
* **dev:** bootstrap a session and keep the advisory gates offline
([17a47bb](https://github.com/Archont561/clifacet/commit/17a47bb2709f3a0b3f19bc17107f48cbed2392fa))
* **hooks:** lint on commit, audit on push, validate the message
([3dcb4e9](https://github.com/Archont561/clifacet/commit/3dcb4e934509e6831a296287806b106460452f18))
* **release:** bump every recorded version with one tested command
([5c6d101](https://github.com/Archont561/clifacet/commit/5c6d1010ba20d53e7fbbfeab7163556b8f0c4c5d))

### Fixed

* **prompt:** refuse a dialog when stdin is not a terminal
([79a261e](https://github.com/Archont561/clifacet/commit/79a261e28709673bad6f8b2dc0e41129ae48136c))
* **ci:** restore the import the advisory-cache adapter needs
([86f1218](https://github.com/Archont561/clifacet/commit/86f121856d4eb9a5c469480cb1e95eb7ac330119))
* **tooling:** scope the documented path to the section that documents it
([8e01b0f](https://github.com/Archont561/clifacet/commit/8e01b0fecfe4b3ffb302161a6ea4223421ec4237))
* **tooling:** make the offline gate offline, and correct two stale claims
([ba15ca6](https://github.com/Archont561/clifacet/commit/ba15ca6cbc1b79c00be22b758b09a3b6eb6bdef2))
* **tui:** stop a form with no keyboard from waiting for a key
([62a6d6d](https://github.com/Archont561/clifacet/commit/62a6d6d8b99e1d04fc423645632522a5b7fc5b1a))
* **release:** date a first release and prove the notes before publishing
([5af2e6d](https://github.com/Archont561/clifacet/commit/5af2e6dcf2519fb54df86daefc422e365cfbd966))
* **dev:** stop forwarding bootstrap flags to the restore launcher
([e8c4342](https://github.com/Archont561/clifacet/commit/e8c43424b920a572b389b1110b7327f2b6f983c5))
* **dev:** force the sandbox branch refresh a restore needs
([1a953e7](https://github.com/Archont561/clifacet/commit/1a953e7b715f81374a9cba49556381ecec718bc7))
* **ci:** package each crate only when its registry deps resolve
([afcf91b](https://github.com/Archont561/clifacet/commit/afcf91baf37ba6909e5466b39e29b8d9b9430d07))
* **ci:** pin setup-pixi with the required v-prefixed version
([beb47d6](https://github.com/Archont561/clifacet/commit/beb47d682dc037a9fff8770fbb06467e04ec8569))

### Changed

* **ci:** move the workflow decisions into tested scripts
([c922cd4](https://github.com/Archont561/clifacet/commit/c922cd4e691aad93feb45742b62315b6b89020e4))
* **example:** move the reference application into its own crate
([3178b1d](https://github.com/Archont561/clifacet/commit/3178b1d44db717a4b242b1235cacf798e9d6e2f0))

[Unreleased]: https://github.com/Archont561/clifacet/compare/v0.1.0...HEAD
