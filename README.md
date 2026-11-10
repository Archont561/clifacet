# Offline sandbox (orphan branch)

Built 2026-11-10T09:16:42Z from commit `bdc2804` for platform `linux-64`.
`pixi.lock` sha256 `e9999a96acbef2a6928ea1bad5363e8be3808b57a82f32456bdb2eae848cf030`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 575.1 MiB | 2245.5 MiB | 72 |

Cargo dependencies: **284 crates**, 395.8 MiB (loose) from `Cargo.lock` sha256 `135dd28a1772…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
