use std::fs;

use crate::common::TestContext;
use crate::rum_snapshot;

#[test]
fn removes_single_file() {
    let ctx = TestContext::new();
    let p = ctx.make_file("a.txt", 256);

    rum_snapshot!(ctx.command().arg(&p), @r"
    success: true
    exit_code: 0
    ----- stdout -----

    ----- stderr -----
    ");

    assert!(!p.exists());
}

#[test]
fn removes_large_file() {
    let ctx = TestContext::new();
    let p = ctx.make_file("big.bin", 64 * 1024 * 1024);
    assert_eq!(fs::metadata(&p).unwrap().len(), 64 * 1024 * 1024);

    ctx.command().arg(&p).assert().success();

    assert!(!p.exists());
}

#[test]
fn removes_many_files_in_one_invocation() {
    let ctx = TestContext::new();
    let paths: Vec<_> = (0..20)
        .map(|i| ctx.make_file(&format!("f{i}.bin"), 1024))
        .collect();

    ctx.command().args(&paths).assert().success();

    for p in &paths {
        assert!(!p.exists(), "{} should be gone", p.display());
    }
}

#[test]
fn verbose_prints_each_removal() {
    let ctx = TestContext::new();
    let p = ctx.make_file("note.txt", 16);

    let output = ctx.command().arg("-v").arg(&p).output().expect("spawn");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("removed"),
        "expected 'removed' in output, got: {stdout}"
    );
    assert!(stdout.contains("note.txt"));
    assert!(!p.exists());
}

#[test]
fn removes_symlink_not_target() {
    let ctx = TestContext::new();
    let target = ctx.make_file("target", 1024);
    let link = ctx.root.join("link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, &link).unwrap();

    ctx.command().arg(&link).assert().success();

    assert!(!link.exists());
    assert!(target.exists(), "symlink target must survive");
}
