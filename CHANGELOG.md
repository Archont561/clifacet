# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
This file is generated: `pixi run changelog` renders it from the conventional
commits with `convco`, and `pixi run changelog-check` fails when the committed
copy has drifted. Edit the commit messages, not the file.

## [Unreleased]

## [0.10.0] - 2026-11-22

### Added

* **async:** dispatch MCP tool calls asynchronously and add http-lite acceptance example (CLI-86,
CLI-87)
([2367805](https://github.com/Archont561/clifacet/commit/2367805797515bfede1efdf2593edfd0ee30badb))
* **tui:** implement EventStream select loop and async choice providers (CLI-85)
([a7a80d9](https://github.com/Archont561/clifacet/commit/a7a80d9b9bf50c3b395d788c707da1393fcd84ee))
* **async:** implement frontend progress rendering across execution modes (CLI-84)
([ffae546](https://github.com/Archont561/clifacet/commit/ffae54693fad7cd7f623bff43c6783694655f60c))
* **async:** implement AsyncPlan and async dry-run preview (CLI-83)
([1dee8ed](https://github.com/Archont561/clifacet/commit/1dee8eddfd48aef0be64741c3de79b074524b1fb))
* implement cancellation contract Ctrl-C grace period and timeout (CLI-82)
([c870235](https://github.com/Archont561/clifacet/commit/c870235add7a4df256400aa23dd7cfa97c83093a))
* support async methods in commands macro and lazy async context (CLI-81)
([25a8b06](https://github.com/Archont561/clifacet/commit/25a8b066e4e180dc7fbceece3ca9d2e61355a37c))

## [0.9.0] - 2026-11-21

### Added

* **async:** define core async traits, cancellation, and progress contexts (CLI-79)
([31074a3](https://github.com/Archont561/clifacet/commit/31074a3e13989ee33ccfb5fa82c397bf614e15e8))

## [0.8.0] - 2026-11-20

### Added

* **completions:** implement context-aware dynamic shell completions (CLI-78)
([20b762e](https://github.com/Archont561/clifacet/commit/20b762eb3576260ca47c917e4423295c382b6c81))
* **mcp:** implement Model Context Protocol stdio tool server (CLI-77)
([8912831](https://github.com/Archont561/clifacet/commit/8912831081b390a634ba7900f0c7bfb7742a2710))
* **skill:** implement write-skill command with drift check (CLI-76)
([6c9cde7](https://github.com/Archont561/clifacet/commit/6c9cde781a15311cb9fe53739e3209bea878a0ed))
* **skill:** implement MiniJinja skill renderer and embedded template (CLI-75)
([9df5115](https://github.com/Archont561/clifacet/commit/9df511577596a8476a4517d4ce0893f9238e9b14))
* **skill:** define pluggable SkillRenderer trait and feature boundary (CLI-74)
([fb023ea](https://github.com/Archont561/clifacet/commit/fb023ea9c55942a991b50e7afd81903d380237b9))
* **schema:** implement machine-readable CLI schema introspection (CLI-73)
([cfc3584](https://github.com/Archont561/clifacet/commit/cfc3584a84e7ae142ec6fc07b1a49488b91023c6))
* **cli:** structured JSON execution mode and standardized failure schema (CLI-72)
([c2db237](https://github.com/Archont561/clifacet/commit/c2db237430ace6f1b6fda0c938262eba1822afff))
* **cli:** retire build api and directory scanning (CLI-71)
([fca061c](https://github.com/Archont561/clifacet/commit/fca061cb0243bddf7f5f1bd06f6d69b3e099f6df))
* **cli:** implement commands and group macros with context extractors (CLI-69, CLI-70)
([18f60c1](https://github.com/Archont561/clifacet/commit/18f60c1fff37dd5d84c89bc780036aada39ce4e8))
* **docs:** adopt Fumadocs static documentation and GitHub Pages publishing
([92794b4](https://github.com/Archont561/clifacet/commit/92794b4b367b332b6f1cfefd3f4b4a9c829707f7))

## [0.7.0] - 2026-11-19

### Added

* **tui:** expand form with multi-value collectors and choice/toggle widgets (CLI-62)
([3dc0128](https://github.com/Archont561/clifacet/commit/3dc0128fd34112d4451fe8d64bceb7687434154f))
* **settings:** cascading configuration presets and line citations (CLI-60, CLI-61)
([d6acd0c](https://github.com/Archont561/clifacet/commit/d6acd0c8f93621dba205efb221d9d7bf73f143c2))

### Fixed

* **docs:** resolve clippy warnings in plugin snippet (CLI-29)
([c4a4b64](https://github.com/Archont561/clifacet/commit/c4a4b642bd81c869265c55be18c6f6da9aeac1bc))

## [0.6.0] - 2026-11-18

### Added

* **settings:** implement Figment configuration engine and command function mapping (CLI-59)
([57d270f](https://github.com/Archont561/clifacet/commit/57d270f49c257630201c89025bd9026a5b729264))
* **settings:** declare figment dependency and decision-22 backlog
([bba183d](https://github.com/Archont561/clifacet/commit/bba183d6a48ea2ffcbdaa24681b3039a02de4b0b))

### Changed

* **tools:** adopt canonical tasknest automation and reorganize scripts
([41664fe](https://github.com/Archont561/clifacet/commit/41664fe62a5f24387be36e3de566ed318f096eac))

## [0.5.0] - 2026-11-10

### Added

* **automation:** one doit task graph behind `pixi run automation`
([b66e8fa](https://github.com/Archont561/clifacet/commit/b66e8fa8da605e35ad583638f556a8fdece2104e))
* **macros:** register a command folder with one attribute
([84d7ea2](https://github.com/Archont561/clifacet/commit/84d7ea2dad7602f2c2700b2d9dea09dc3e835936))
* write backlog-lite and the reference application as functions
([71e46c1](https://github.com/Archont561/clifacet/commit/71e46c1065a6f55fea9b42c89c5be37f9941d288))
* write a command as a function
([f9e842d](https://github.com/Archont561/clifacet/commit/f9e842d9ecf11697b5c53ce7fd5a90dc159f9bc7))
* **settings:** layer a config file under the flags and the environment
([f4609bf](https://github.com/Archont561/clifacet/commit/f4609bf6f0ba00294a3a26e34076639335c5cd41))
* **backlog:** check that a task's status agrees with its criteria
([98b833d](https://github.com/Archont561/clifacet/commit/98b833dc1139b3ae1651ab12fab6828aabf81ab4))
* **check:** lint the prose with Vale, and make it pass
([6610272](https://github.com/Archont561/clifacet/commit/66102724b8ffe2fbf223aa13fa9bf3f7d30091ae))
* **check:** annotate the prose finding on the line it is about
([e0f0c59](https://github.com/Archont561/clifacet/commit/e0f0c59003f1a24f70e746df8103158dd0b14e90))
* **check:** report what Vale found where a review can read it
([cb464be](https://github.com/Archont561/clifacet/commit/cb464be1ad0d237b491c066fe8cd199afe8050f5))
* **check:** lint the prose with Vale, keep alint for the repository
([bbff149](https://github.com/Archont561/clifacet/commit/bbff149028fce732e455e3606a5baa08cc243ba8))
* **command:** discover a folder's commands from its build script
([f42451f](https://github.com/Archont561/clifacet/commit/f42451fd45011a6d9a5de2148c278712ccaadccc))

### Fixed

* **ci:** install markdownlint from the lockfile instead of fetching it
([4001ddd](https://github.com/Archont561/clifacet/commit/4001ddd8e932958da7bb58e26f9767101e05ab1b))
* **ci:** run the page checks on every change, not only documentation ones
([8c326e2](https://github.com/Archont561/clifacet/commit/8c326e21e98fe09539af882a3ece48c8c2701a77))
* **automation:** make the coverage task reach its report
([2965db7](https://github.com/Archont561/clifacet/commit/2965db7d964bffe42ceb562456d2bf489364d58f))
* **automation:** run backlog-check through doit, not a phantom binary
([171de6f](https://github.com/Archont561/clifacet/commit/171de6f4ed521551022e2e99d16a59ca80a70a79))
* **macros:** name the folder, not its path, when it cannot be included
([eaca858](https://github.com/Archont561/clifacet/commit/eaca858a51a5ba13bca33c7325b0de4c27cad6cd))
* **macros:** name the folder's file relative to the crate in messages
([cc76070](https://github.com/Archont561/clifacet/commit/cc76070eb9c3b4866b7da700844083c34c25b63f))
* **macros:** write the folder module inline so the MSRV accepts it
([b7a790e](https://github.com/Archont561/clifacet/commit/b7a790ef0f27b9bc42043c7ce9442640fffef50d))
* **macros:** build the fixture path from parts so Windows finds it
([890923d](https://github.com/Archont561/clifacet/commit/890923d39f55eaf2f0ce87d17baff7ff84c62588))
* **settings:** ask the platform where its config directory is
([5e1b4f2](https://github.com/Archont561/clifacet/commit/5e1b4f212d75b123c4cc2341494d3a50d5015060))

## [0.4.0] - 2026-11-08

### Added

* **backlog-lite:** run the example on the runner, with a context
([0ecc2c9](https://github.com/Archont561/clifacet/commit/0ecc2c96f507b3c6f73b65d44d0c39e6ff73275f))
* register a folder of commands in one line
([ee8b93d](https://github.com/Archont561/clifacet/commit/ee8b93df81bcd4966b519b7e73bf4c5dc5b8ec0f))
* give commands the application's context
([2a993f4](https://github.com/Archont561/clifacet/commit/2a993f46eadffd1d6ff3707f603ca2ce0e78ee4b))
* run an application through clifacet::App
([fab54ba](https://github.com/Archont561/clifacet/commit/fab54ba989497736d477016bf186c0126fc43c42))

### Fixed

* **release:** keep the changelog the release commits from going stale
([b30fa4f](https://github.com/Archont561/clifacet/commit/b30fa4fe5ea69381ff7c916078cb0f05a368b87e))

### Changed

* **example:** run the reference application through clifacet::App
([5227c65](https://github.com/Archont561/clifacet/commit/5227c65543767987befc07fb819098bb334f8e95))

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

[Unreleased]: https://github.com/Archont561/clifacet/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/Archont561/clifacet/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/Archont561/clifacet/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/Archont561/clifacet/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/Archont561/clifacet/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Archont561/clifacet/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Archont561/clifacet/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Archont561/clifacet/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/Archont561/clifacet/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/Archont561/clifacet/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Archont561/clifacet/compare/v0.1.0...v0.2.0
