use crate::common::TestContext;

#[test]
fn recursive_removes_small_tree() {
    let ctx = TestContext::new();
    let tree = ctx.make_tree("tree", 3, 2, 4, 512);

    ctx.command().arg("-r").arg(&tree).assert().success();

    assert!(!tree.exists());
}

#[test]
fn recursive_removes_large_flat_dir() {
    let ctx = TestContext::new();
    let dir = ctx.make_dir("flat");
    for i in 0..500 {
        ctx.make_file(&format!("flat/f{i}"), 4 * 1024);
    }

    ctx.command().arg("-r").arg(&dir).assert().success();

    assert!(!dir.exists());
}

#[test]
fn recursive_removes_deep_tree() {
    let ctx = TestContext::new();
    let tree = ctx.make_tree("deep", 2, 5, 3, 128);

    ctx.command().arg("-r").arg(&tree).assert().success();

    assert!(!tree.exists());
}

#[test]
fn recursive_capital_r_alias() {
    let ctx = TestContext::new();
    let dir = ctx.make_tree("cap", 2, 2, 2, 64);

    ctx.command().arg("-R").arg(&dir).assert().success();

    assert!(!dir.exists());
}

#[test]
fn removes_empty_directory() {
    let ctx = TestContext::new();
    let dir = ctx.make_dir("empty");

    ctx.command().arg("-r").arg(&dir).assert().success();

    assert!(!dir.exists());
}
