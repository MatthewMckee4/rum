# Contributing to rum

Thanks for your interest in contributing. Small fixes can go straight to a
PR; for anything larger please open an issue first so we can agree on the
approach.

## Prerequisites

- Install the [Rust toolchain](https://www.rust-lang.org/tools/install).
- Optional but recommended:
  - [just](https://github.com/casey/just) — task runner used by the commands
    below.
  - [prek](https://github.com/j178/prek) — runs pre-commit hooks locally.
  - [cargo-nextest](https://nexte.st) — parallel test runner (`just test`
    falls back to `cargo test` when absent).

```bash
uv tool install prek   # optional
prek install           # optional — auto-run hooks on commit
```

## Development

```bash
just test         # run all tests
just test <name>  # run a single test
just lint         # cargo fmt --check + cargo clippy
just fmt          # cargo fmt
just bench        # criterion benchmarks vs `/bin/rm`
just pre-commit   # run all pre-commit hooks (uvx prek run -a)
```

Integration tests live under `tests/it/` following the single-binary
layout described by [matklad][matklad-it]. Each test drives the built
`rum` binary through `assert_cmd` and asserts on the filesystem state
and/or a filtered `insta` snapshot of stdout/stderr.

New functional behavior should come with an integration test.

## GitHub Actions

If you are updating workflows, run [`pinact`][pinact] to pin action
versions to immutable commit SHAs.

```bash
pinact run
```

[matklad-it]: https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html
[pinact]: https://github.com/suzuki-shunsuke/pinact
