#![allow(dead_code, unreachable_pub)]

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use assert_fs::fixture::ChildPath;
use regex::Regex;

/// Test context for running rum commands in an isolated temp directory.
pub struct TestContext {
    pub root: ChildPath,
    filters: Vec<(String, String)>,
    _root: tempfile::TempDir,
}

impl TestContext {
    pub fn new() -> Self {
        let root = tempfile::TempDir::with_prefix("rum-test")
            .expect("failed to create test root directory");

        let mut filters = Vec::new();
        filters.extend(
            Self::path_patterns(root.path())
                .into_iter()
                .map(|pat| (pat, "[TEMP]/".to_string())),
        );

        Self {
            root: ChildPath::new(root.path()),
            _root: root,
            filters,
        }
    }

    pub fn filters(&self) -> Vec<(&str, &str)> {
        self.filters
            .iter()
            .map(|(p, r)| (p.as_str(), r.as_str()))
            .chain(INSTA_FILTERS.iter().copied())
            .collect()
    }

    /// A fresh `rum` command rooted at the temp directory.
    pub fn command(&self) -> Command {
        let mut cmd = Command::new(get_bin());
        cmd.current_dir(self.root.path());
        cmd
    }

    /// Create a file at `relative` containing `size` bytes.
    pub fn make_file(&self, relative: &str, size: usize) -> PathBuf {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        write_bytes(&path, size);
        path
    }

    /// Create a directory at `relative`.
    pub fn make_dir(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(&path).expect("create dir");
        path
    }

    /// Build a nested tree under `relative`.
    pub fn make_tree(
        &self,
        relative: &str,
        wide: usize,
        deep: usize,
        files_per_dir: usize,
        file_size: usize,
    ) -> PathBuf {
        let root = self.root.join(relative);
        build_tree(&root, wide, deep, files_per_dir, file_size);
        root
    }

    fn path_patterns(path: impl AsRef<Path>) -> Vec<String> {
        let mut patterns = Vec::new();
        if path.as_ref().exists() {
            patterns.push(Self::path_pattern(
                path.as_ref().canonicalize().expect("canonicalize"),
            ));
        }
        patterns.push(Self::path_pattern(path));
        patterns
    }

    fn path_pattern(path: impl AsRef<Path>) -> String {
        format!(
            r"{}(\\|\/)*",
            regex::escape(&dunce::simplified(path.as_ref()).display().to_string())
                .replace(r"\\", r"(\\|\/)+")
        )
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

fn write_bytes(path: &Path, size: usize) {
    let mut f = File::create(path).expect("create file");
    if size == 0 {
        return;
    }
    let chunk = vec![0xABu8; size.min(64 * 1024)];
    let mut remaining = size;
    while remaining > 0 {
        let n = remaining.min(chunk.len());
        f.write_all(&chunk[..n]).expect("write chunk");
        remaining -= n;
    }
}

fn build_tree(root: &Path, wide: usize, deep: usize, files_per_dir: usize, file_size: usize) {
    std::fs::create_dir_all(root).expect("create tree root");
    for i in 0..files_per_dir {
        write_bytes(&root.join(format!("f{i}.bin")), file_size);
    }
    if deep == 0 {
        return;
    }
    for i in 0..wide {
        build_tree(
            &root.join(format!("d{i}")),
            wide,
            deep - 1,
            files_per_dir,
            file_size,
        );
    }
}

pub fn get_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rum"))
}

pub static INSTA_FILTERS: &[(&str, &str)] = &[
    (r"\r\n", "\n"),
    // Strip ANSI color codes.
    (r"[\x1b]\[[0-9;]*m", ""),
    // Normalize `rum X.Y.Z` in --version output.
    (r"rum \d+\.\d+\.\d+(-[a-z]+\.\d+)?", "rum [VERSION]"),
    // Strip trailing OS-error codes that vary between Linux / macOS wording.
    (r"\(os error \d+\)", "(os error N)"),
];

pub fn apply_filters<T: AsRef<str>>(mut snapshot: String, filters: impl AsRef<[(T, T)]>) -> String {
    for (matcher, replacement) in filters.as_ref() {
        let re = Regex::new(matcher.as_ref()).expect("compile filter");
        if re.is_match(&snapshot) {
            snapshot = re.replace_all(&snapshot, replacement.as_ref()).to_string();
        }
    }
    snapshot
}

#[allow(clippy::print_stderr)]
pub fn run_and_format(
    cmd: &mut Command,
    filters: &[(&str, &str)],
) -> (String, std::process::Output) {
    let program = cmd.get_program().to_string_lossy().to_string();
    let output = cmd
        .output()
        .unwrap_or_else(|err| panic!("failed to spawn {program}: {err}"));

    eprintln!("\n━━━━━━━━━━━━━━━━━━━━ Unfiltered output ━━━━━━━━━━━━━━━━━━━━");
    eprintln!(
        "----- stdout -----\n{}\n----- stderr -----\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    eprintln!("────────────────────────────────────────────────────────────\n");

    let snapshot = apply_filters(
        format!(
            "success: {:?}\nexit_code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
            output.status.success(),
            output.status.code().unwrap_or(!0),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        ),
        filters,
    );

    (snapshot, output)
}

#[macro_export]
macro_rules! rum_snapshot {
    ($cmd:expr, @$snapshot:literal) => {{
        $crate::rum_snapshot!($crate::common::INSTA_FILTERS.to_vec(), $cmd, @$snapshot)
    }};
    ($filters:expr, $cmd:expr, @$snapshot:literal) => {{
        let (snapshot, output) = $crate::common::run_and_format($cmd, &$filters);
        ::insta::assert_snapshot!(snapshot, @$snapshot);
        output
    }};
}

#[allow(unused_imports)]
pub(crate) use rum_snapshot;
