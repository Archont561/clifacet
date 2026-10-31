# Offline sandbox (orphan branch)

Built 2026-10-31T01:58:27Z from commit `3cd773a` for platform `linux-64`.
`pixi.lock` sha256 `a49f79f000b7fd84ed53cc3c289f1fef24f962d4676bc3f7c4c3ac525ef81730`.

The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/linux-64/pixi-sandbox`. The branch root intentionally contains documentation only.

| env | platform | packed | unpacked | files |
| --- | --- | ---: | ---: | ---: |
| `default` | linux-64 | 563.6 MiB | 2188.6 MiB | 67 |

Cargo dependencies: **284 crates**, 395.8 MiB (loose) from `Cargo.lock` sha256 `f9329af966f5…`; restore materialises them to `.pixi-sandbox/vendor/`. Built with cargo 1.99.1 (5f94df478 2026-08-27); rustc 1.99.1 (58cf8f9c3 2026-10-13).


## Restore on the disconnected machine

```bash
./.pixi-sandbox/tools/linux-64/pixi-sandbox doctor --branch-location . --verify
./.pixi-sandbox/tools/linux-64/pixi-sandbox restore --branch-location . --output-path <project> --force
# then, from <project> with no network, use pixi as the sole entrypoint:
pixi install --frozen --offline
pixi run --frozen -- cargo build --offline
```

Every manifest blob is verified before it is written into the working tree.
