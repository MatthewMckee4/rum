use crate::common::TestContext;
use crate::rum_snapshot;

#[test]
fn missing_file_without_force_errors() {
    let ctx = TestContext::new();

    rum_snapshot!(ctx.filters(), ctx.command().arg("does-not-exist"), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    rum: cannot remove 'does-not-exist': No such file or directory (os error N)
    ");
}

#[test]
fn missing_file_with_force_is_silent() {
    let ctx = TestContext::new();

    rum_snapshot!(ctx.command().arg("-f").arg("does-not-exist"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");
}

#[test]
fn directory_without_recursive_errors() {
    let ctx = TestContext::new();
    let dir = ctx.make_dir("subdir");

    rum_snapshot!(ctx.filters(), ctx.command().arg(&dir), @r"
    success: false
    exit_code: 1
    ----- stdout -----

    ----- stderr -----
    rum: cannot remove '[TEMP]/subdir': is a directory
    ");

    assert!(dir.exists(), "directory must survive without -r");
}

#[test]
fn continues_after_individual_failure() {
    let ctx = TestContext::new();
    let good = ctx.make_file("good.txt", 16);

    let output = ctx
        .command()
        .arg("missing-one")
        .arg(&good)
        .output()
        .expect("spawn");

    assert!(!output.status.success(), "exit code must indicate failure");
    assert!(!good.exists(), "good file must still be removed");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("missing-one"), "stderr: {stderr}");
}

#[test]
fn no_operand_without_force_errors() {
    let ctx = TestContext::new();

    rum_snapshot!(&mut ctx.command(), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    error: the following required arguments were not provided:
      [PATHS]...

    Usage: rum [PATHS]...

    For more information, try '--help'.
    ");
}

#[test]
fn no_operand_with_force_is_silent() {
    let ctx = TestContext::new();

    rum_snapshot!(ctx.command().arg("-f"), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");
}
