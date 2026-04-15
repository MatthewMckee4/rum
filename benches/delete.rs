//! Benchmarks comparing `rum` against the system `rm` binary.
//!
//! Each iteration builds a fresh tree in a tempdir (excluded from timing via
//! `iter_batched`) and then times only the deletion.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use rum::{Options, remove_path};
use tempfile::TempDir;

fn make_file(path: &Path, size: usize) {
    let mut f = File::create(path).unwrap();
    if size > 0 {
        let chunk = vec![0u8; size.min(64 * 1024)];
        let mut remaining = size;
        while remaining > 0 {
            let n = remaining.min(chunk.len());
            f.write_all(&chunk[..n]).unwrap();
            remaining -= n;
        }
    }
}

fn make_tree(root: &Path, wide: usize, deep: usize, files: usize, file_size: usize) {
    fs::create_dir_all(root).unwrap();
    for i in 0..files {
        make_file(&root.join(format!("f{i}.bin")), file_size);
    }
    if deep == 0 {
        return;
    }
    for i in 0..wide {
        make_tree(
            &root.join(format!("d{i}")),
            wide,
            deep - 1,
            files,
            file_size,
        );
    }
}

struct Fixture {
    _tmp: TempDir,
    target: PathBuf,
}

fn build_empty_file() -> Fixture {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("empty");
    make_file(&target, 0);
    Fixture { _tmp: tmp, target }
}

fn build_single_large(size: usize) -> Fixture {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("big.bin");
    make_file(&target, size);
    Fixture { _tmp: tmp, target }
}

fn build_flat(files: usize, size: usize) -> Fixture {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("root");
    fs::create_dir(&target).unwrap();
    for i in 0..files {
        make_file(&target.join(format!("f{i}")), size);
    }
    Fixture { _tmp: tmp, target }
}

fn build_tree(wide: usize, deep: usize, files: usize, size: usize) -> Fixture {
    let tmp = TempDir::new().unwrap();
    let target = tmp.path().join("root");
    make_tree(&target, wide, deep, files, size);
    Fixture { _tmp: tmp, target }
}

fn recursive_opts() -> Options {
    Options {
        recursive: true,
        ..Options::default()
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "inline scenarios keep the bench readable at the call site"
)]
fn bench_delete(c: &mut Criterion) {
    let rm = which_rm();
    let mut group = c.benchmark_group("delete");
    group.sample_size(10);

    group.bench_function("rum/empty-file", |b| {
        b.iter_batched(
            build_empty_file,
            |f| remove_path(&f.target, Options::default()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/empty-file", |b| {
        b.iter_batched(
            build_empty_file,
            |f| run_rm(&rm, &["-f"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.bench_function("rum/single-large-256MiB", |b| {
        b.iter_batched(
            || build_single_large(256 * 1024 * 1024),
            |f| remove_path(&f.target, Options::default()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/single-large-256MiB", |b| {
        b.iter_batched(
            || build_single_large(256 * 1024 * 1024),
            |f| run_rm(&rm, &["-f"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.bench_function("rum/flat-500x0B", |b| {
        b.iter_batched(
            || build_flat(500, 0),
            |f| remove_path(&f.target, recursive_opts()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/flat-500x0B", |b| {
        b.iter_batched(
            || build_flat(500, 0),
            |f| run_rm(&rm, &["-rf"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.bench_function("rum/flat-2000x8KiB", |b| {
        b.iter_batched(
            || build_flat(2000, 8 * 1024),
            |f| remove_path(&f.target, recursive_opts()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/flat-2000x8KiB", |b| {
        b.iter_batched(
            || build_flat(2000, 8 * 1024),
            |f| run_rm(&rm, &["-rf"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.bench_function("rum/flat-10000x256B", |b| {
        b.iter_batched(
            || build_flat(10_000, 256),
            |f| remove_path(&f.target, recursive_opts()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/flat-10000x256B", |b| {
        b.iter_batched(
            || build_flat(10_000, 256),
            |f| run_rm(&rm, &["-rf"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.bench_function("rum/tree-4w3d20f-4KiB", |b| {
        b.iter_batched(
            || build_tree(4, 3, 20, 4 * 1024),
            |f| remove_path(&f.target, recursive_opts()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/tree-4w3d20f-4KiB", |b| {
        b.iter_batched(
            || build_tree(4, 3, 20, 4 * 1024),
            |f| run_rm(&rm, &["-rf"], &f.target),
            BatchSize::PerIteration,
        );
    });

    // Very deep tree: exercises recursion / fd churn.
    group.bench_function("rum/tree-2w6d5f-1KiB", |b| {
        b.iter_batched(
            || build_tree(2, 6, 5, 1024),
            |f| remove_path(&f.target, recursive_opts()).unwrap(),
            BatchSize::PerIteration,
        );
    });
    group.bench_function("rm/tree-2w6d5f-1KiB", |b| {
        b.iter_batched(
            || build_tree(2, 6, 5, 1024),
            |f| run_rm(&rm, &["-rf"], &f.target),
            BatchSize::PerIteration,
        );
    });

    group.finish();
}

fn which_rm() -> PathBuf {
    for p in ["/bin/rm", "/usr/bin/rm"] {
        if Path::new(p).exists() {
            return PathBuf::from(p);
        }
    }
    PathBuf::from("rm")
}

fn run_rm(rm: &Path, args: &[&str], target: &Path) {
    let status = Command::new(rm).args(args).arg(target).status().unwrap();
    assert!(status.success(), "rm failed");
}

criterion_group!(benches, bench_delete);
criterion_main!(benches);
