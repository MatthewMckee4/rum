use crate::common::TestContext;
use crate::rum_snapshot;

#[test]
fn version_long_flag() {
    let ctx = TestContext::new();
    rum_snapshot!(ctx.command().arg("--version"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    rum [VERSION]

    ----- stderr -----
    ");
}

#[test]
fn version_short_flag() {
    let ctx = TestContext::new();
    rum_snapshot!(ctx.command().arg("-V"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    rum [VERSION]

    ----- stderr -----
    ");
}
