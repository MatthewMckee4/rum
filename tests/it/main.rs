//! Single integration-test binary, as documented by matklad in
//! <https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html>.
//!
//! Everything here exercises the `rum` binary end-to-end via `assert_cmd`.

pub(crate) mod common;

mod help;
mod remove;
mod version;
