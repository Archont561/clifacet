# Offline sandbox (orphan branch)

Built 2026-11-21T14:22:46Z from commit `2135444` for platform `linux-64`.
`pixi.lock` sha256 `27e5e4bf3936b1a5d17f622f9d98652d5f1f131b2f4d9ef24564ba2ad2289db2`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 583.4 MiB | 2264.5 MiB | 70 |

Cargo dependencies: **290 crates**, 397.5 MiB (loose) from `Cargo.lock` sha256 `757d2d1369a9…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.100.0 (3acc79c0c 2026-11-05); rustc 1.100.0 (0ef1bc386 2026-11-09).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
