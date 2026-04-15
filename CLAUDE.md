- ALWAYS read CONTRIBUTING.md for guidelines on how to run tools.
- ALWAYS attempt to add a test case for changed behavior.
- PREFER integration tests at `tests/it/...` over library unit tests — we
  follow matklad's single-integration-binary layout.
- ALWAYS use `rum_snapshot!` for CLI output tests. Assert on filesystem
  state separately (files gone, siblings preserved, etc.).
- ALWAYS run `just test` after making changes.
- ALWAYS run `uvx prek run -a` at the end of a task.
- The main goal of this project is to be faster than `rm`. Any feature or
  abstraction that slows the common `rm -rf` path down needs to justify
  itself against a benchmark.
- AVOID `panic!`, `unreachable!`, `.unwrap()`, and `unsafe` in production
  code. The crate has `#![forbid(unsafe_code)]` — keep it that way; use
  `rustix` rather than raw libc.
- PREFER `#[expect(...)]` over `#[allow(...)]` when a lint must be
  disabled.
- FOLLOW the existing module split: cross-platform façade in `src/lib.rs`,
  unix fast path in `unix_fast`, std-only fallback in `fallback`.
- Don't add commentary comments. Explain invariants, not mechanics.
