use crate::common::TestContext;
use crate::rum_snapshot;

#[test]
fn help_long_flag() {
    let ctx = TestContext::new();
    rum_snapshot!(ctx.command().arg("--help"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    A fast rm replacement written in Rust

    Usage: rum [OPTIONS] [PATHS]...

    Arguments:
      [PATHS]...  Files or directories to remove

    Options:
      -r, --recursive  Remove directories and their contents recursively
      -f, --force      Ignore nonexistent files and arguments, never prompt
      -v, --verbose    Explain what is being done
      -h, --help       Print help
      -V, --version    Print version

    ----- stderr -----
    ");
}

#[test]
fn help_short_flag() {
    let ctx = TestContext::new();
    rum_snapshot!(ctx.command().arg("-h"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    A fast rm replacement written in Rust

    Usage: rum [OPTIONS] [PATHS]...

    Arguments:
      [PATHS]...  Files or directories to remove

    Options:
      -r, --recursive  Remove directories and their contents recursively
      -f, --force      Ignore nonexistent files and arguments, never prompt
      -v, --verbose    Explain what is being done
      -h, --help       Print help
      -V, --version    Print version

    ----- stderr -----
    ");
}
