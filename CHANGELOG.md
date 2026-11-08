# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
This file is generated: `pixi run changelog` renders it from the conventional
commits with `convco`, and `pixi run changelog-check` fails when the committed
copy has drifted. Edit the commit messages, not the file.

## [0.3.1] - 2026-11-08

### Added

* **macros:** declare a command tree where its commands live
([b6e9691](https://github.com/Archont561/clifacet/commit/b6e9691e1eb5f3fdd24a09450fb34085d15b3899))

### Fixed

* **deps:** depend on clap without its default features
([d7bc6e4](https://github.com/Archont561/clifacet/commit/d7bc6e4a5a62cd64ed60902b3d93da049daadd75))
* hide a secret's environment value from --help
([885403c](https://github.com/Archont561/clifacet/commit/885403c4d96808c21ac58f8e9692dcf01b3c86c1))
* **release:** commit the changelog as the workflow's bot
([4e2f139](https://github.com/Archont561/clifacet/commit/4e2f139e3a2c2d39c5561ac33d537cbddba3f4bf))

### Changed

* **example:** lay out one file per command and drop the demos
([e240d0e](https://github.com/Archont561/clifacet/commit/e240d0e07765eead656169d9284d8760af63e7db))
* **backlog-lite:** lay out one file per command
([e4a2366](https://github.com/Archont561/clifacet/commit/e4a2366639e9f1b3af3355dcb17de41a92aa98a0))
* **backlog-lite:** declare the tree instead of hand-writing it
([4581a0a](https://github.com/Archont561/clifacet/commit/4581a0a0f8c15457ad98c1363081361fa1fa8712))

## [0.3.0] - 2026-11-06

### ⚠ BREAKING CHANGE

* Session::confirm returns Confirmation instead of bool; call
.is_confirmed() where a boolean is wanted.

* the json feature is removed; enable serde instead and
depend on serde_json in the application.

### Added

* **command:** plan a command's work so a dry run and a real run share a path
([1093668](https://github.com/Archont561/clifacet/commit/1093668433e4b63cbd55afadfbaed420201d47ce))
* **report:** write a failure or clap's verdict and answer with its code
([10a64d6](https://github.com/Archont561/clifacet/commit/10a64d6328fddb773ce46c22fcb80cd939192ce3))
* **provenance:** derive where each value came from from the parse
([ce343f2](https://github.com/Archont561/clifacet/commit/ce343f261d6d1684de7ae89a0630731c3a373e37))
* **example:** offer the level as a choice in the full-screen form
([526dcb6](https://github.com/Archont561/clifacet/commit/526dcb661e383a3f2eaaf4e59a4d787d1468ca1e))
* **macros:** refuse `form` on a list field
([15aebfe](https://github.com/Archont561/clifacet/commit/15aebfe58d6d2097085824f70ce29dc91966fff0))
* **tui:** draw each form field as what its grammar says it is
([be444ff](https://github.com/Archont561/clifacet/commit/be444ff45f1287d1327abdc4ad3c9f94614f2abd))
* **backlog-lite:** ask which status a bare --status filter means
([ab39746](https://github.com/Archont561/clifacet/commit/ab39746157fad24ea6a0d9ca8b777f41f0215f75))
* **prompt:** collect a list by repeating one question until the user stops
([dcc080f](https://github.com/Archont561/clifacet/commit/dcc080fffa5c895b3be2c01f0e54914713521d29))
* **prompt:** ask for a secret without echo, and record prompted answers
([91383e7](https://github.com/Archont561/clifacet/commit/91383e724a6636a5468dc3929b7d89adcf5f47fa))
* **prompt:** ask the user to pick one of a fixed set of values
([222318e](https://github.com/Archont561/clifacet/commit/222318e027ec92970e5435c5cb660a423eda4faf))
* **prompt:** say how a confirmation was settled
([f6cdc44](https://github.com/Archont561/clifacet/commit/f6cdc44705d9f5f58431340d09d20cb26d808981))
* retire the reserved json feature in favour of serde
([80e330e](https://github.com/Archont561/clifacet/commit/80e330e90f31203ea0698aeef8b0f885a3bf4c93))
* **failure:** serialise failures and redacted secrets behind an optional serde feature
([d644a0e](https://github.com/Archont561/clifacet/commit/d644a0e9439778d9fe7e654253f71736ccb5cecd))
* **macros:** accept any type the generated impls can name as the input
([ba05b98](https://github.com/Archont561/clifacet/commit/ba05b9847761bcb1df6755d40dded058b99d58b4))

### Fixed

* **tui:** name an action field for what it is when a form refuses it
([524917a](https://github.com/Archont561/clifacet/commit/524917acd111711684d39943e17b6f345fa4b3f7))
* **tui:** mask a declared secret in a form built from the declarations
([55a118d](https://github.com/Archont561/clifacet/commit/55a118d9fbd0682707bbc0743d8980ea9d354410))
* **release:** render the changelog before committing it in the release step
([65b9cbd](https://github.com/Archont561/clifacet/commit/65b9cbd8b5d464d978c6bad90c19a54a9a9457b5))

## [0.2.0] - 2026-11-04

### Added

* **macros:** generate the form and check the whole input from the declarations
([42a49be](https://github.com/Archont561/clifacet/commit/42a49be8c8acb8d87baadb9edbcc0b74db0d05a4))
* **macros:** ask for and normalise a field from its one declaration
([1db9ff8](https://github.com/Archont561/clifacet/commit/1db9ff8f4bf4d9155f7250cb72f25c0b00e31889))
* declare an argument once with a field attribute, and guard every feature's cfg site
([5b700dc](https://github.com/Archont561/clifacet/commit/5b700dc2bc559a950a2156794099a9a440f6a41a))
* **examples:** wrap the backlog.md CLI and record the authoring gaps
([b753b84](https://github.com/Archont561/clifacet/commit/b753b84db784ac647960f619c96e2f29a139fa32))

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

[0.3.1]: https://github.com/Archont561/clifacet/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Archont561/clifacet/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Archont561/clifacet/compare/v0.1.0...v0.2.0
