# rum

A fast `rm` replacement written in Rust.

`rum` is a drop-in replacement for the subset of `rm` that most people
actually use. On Unix it opens each directory once and unlinks children
via `unlinkat(dirfd, name, ...)`, avoiding the full path walk `rm`
performs per file. Work within a directory is fanned out across threads
with [rayon].

## Status

Early. Deliberately minimal feature set — the goal is to be faster
than `rm` on the common cases, not to reach feature parity.

Currently supported flags:

| flag | meaning |
| ------------------- | --------------------------------------------------- |
| `-r`, `-R`, `--recursive` | remove directories and their contents recursively |
| `-f`, `--force` | ignore nonexistent files, never prompt |
| `-v`, `--verbose` | print each path as it is removed |

## Install

```bash
cargo install --path .
```

## Benchmarks

Measured on macOS / APFS with 10 iterations via `cargo bench`. Each
iteration rebuilds the fixture fresh (excluded from timing):

| scenario | rum | `/bin/rm` | speedup |
| --------------------- | --------- | --------- | ------- |
| single 256 MiB file | 1.83 ms | 4.52 ms | 2.5x |
| flat 2000 × 8 KiB | 49.5 ms | 94.3 ms | 1.9x |
| tree 4w × 3d × 20f | 88.7 ms | 104.6 ms | 1.2x |

Run locally with `just bench`.

## Development

```bash
just test       # cargo nextest run (or cargo test)
just lint       # fmt + clippy
just bench      # criterion benchmarks
just pre-commit # uvx prek run -a
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

[rayon]: https://docs.rs/rayon
