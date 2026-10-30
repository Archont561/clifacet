# Clifacet

[![CI](https://github.com/Archont561/clifacet/actions/workflows/ci.yml/badge.svg)](https://github.com/Archont561/clifacet/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/Archont561/clifacet/graph/badge.svg)](https://codecov.io/gh/Archont561/clifacet)
[![crates.io](https://img.shields.io/crates/v/clifacet.svg)](https://crates.io/crates/clifacet)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Archont561/clifacet)

**A Rust library for command-line presentation modes.**

Experimental scaffold: the command core, prompts, the companion derive and the
full-screen TUI are implemented; the JSON frontend is not.
The workspace currently contains `clifacet` and the `clifacet-macros` procedural macro crate.

## Develop

```sh
pixi install
pixi run quality
pixi run test
pixi run test-doc
pixi run test-scripts
pixi run coverage
```

For VS Code or Codespaces, see the [dev container setup](docs/development.md#dev-container).

See [development documentation](docs/development.md), the [docs index](docs/README.md), and the [project backlog](backlog/).

## License

MIT OR Apache-2.0.
