use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub recursive: bool,
    pub force: bool,
    pub verbose: bool,
}

#[derive(Debug)]
pub enum RumError {
    Io { path: PathBuf, source: io::Error },
    IsDirectory { path: PathBuf },
}

impl RumError {
    pub fn path(&self) -> &Path {
        match self {
            Self::Io { path, .. } | Self::IsDirectory { path } => path,
        }
    }
}

impl std::fmt::Display for RumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "cannot remove '{}': {source}", path.display())
            }
            Self::IsDirectory { path } => {
                write!(f, "cannot remove '{}': is a directory", path.display())
            }
        }
    }
}

impl std::error::Error for RumError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::IsDirectory { .. } => None,
        }
    }
}

pub fn remove_paths<I, P>(paths: I, opts: Options) -> Vec<RumError>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut errors = Vec::new();
    for path in paths {
        if let Err(e) = remove_path(path.as_ref(), opts) {
            errors.push(e);
        }
    }
    errors
}

pub fn remove_path(path: &Path, opts: Options) -> Result<(), RumError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if opts.force && e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(RumError::Io {
                path: path.to_path_buf(),
                source: e,
            });
        }
    };

    if meta.is_dir() {
        if !opts.recursive {
            return Err(RumError::IsDirectory {
                path: path.to_path_buf(),
            });
        }
        remove_tree(path, opts)
    } else {
        remove_file_path(path, opts)
    }
}

fn remove_file_path(path: &Path, opts: Options) -> Result<(), RumError> {
    match fs::remove_file(path) {
        Ok(()) => {
            if opts.verbose {
                println!("removed '{}'", path.display());
            }
            Ok(())
        }
        Err(e) if opts.force && e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(RumError::Io {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}

fn log_removed_dir(path: &Path, opts: Options) {
    if opts.verbose {
        println!("removed directory '{}'", path.display());
    }
}

#[cfg(unix)]
mod unix_fast {
    //! Fast unix recursive delete.
    //!
    //! Each directory is opened once (`openat(parent, name, O_DIRECTORY)`),
    //! entries are read via `getdents`, and children are unlinked with
    //! `unlinkat(dirfd, name, …)`. That avoids re-walking the full path on
    //! every child — the biggest cost in `rm -rf` for wide / deep trees.
    //!
    //! Rayon's `try_for_each` fans out per-directory work. Parallelism pays
    //! off for both (a) many files in one directory on APFS/ext4 and (b)
    //! sibling subtrees. Single-subdir recursion skips the fan-out (pure
    //! overhead when there is nothing to spread across threads).
    use std::ffi::{CStr, CString};
    use std::io;
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use rayon::prelude::*;
    use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags};

    use super::{Options, RumError, log_removed_dir};

    fn io_err(path: &Path, err: rustix::io::Errno) -> RumError {
        RumError::Io {
            path: path.to_path_buf(),
            source: io::Error::from_raw_os_error(err.raw_os_error()),
        }
    }

    fn cstring_for(name: &std::ffi::OsStr) -> Result<CString, RumError> {
        CString::new(name.as_bytes()).map_err(|_| RumError::Io {
            path: PathBuf::from(name),
            source: io::Error::new(io::ErrorKind::InvalidInput, "path contains NUL byte"),
        })
    }

    pub fn remove_tree(path: &Path, opts: Options) -> Result<(), RumError> {
        let file_name = path.file_name().ok_or_else(|| RumError::Io {
            path: path.to_path_buf(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"),
        })?;
        let name_c = cstring_for(file_name)?;

        let parent_path = path.parent().and_then(|p| {
            let s = p.as_os_str();
            if s.is_empty() || s == std::ffi::OsStr::new(".") {
                None
            } else {
                Some(p)
            }
        });

        if let Some(pp) = parent_path {
            let parent_fd: OwnedFd =
                match rustix::fs::open(pp, OFlags::DIRECTORY | OFlags::CLOEXEC, Mode::empty()) {
                    Ok(fd) => fd,
                    Err(e) if opts.force && e == rustix::io::Errno::NOENT => return Ok(()),
                    Err(e) => return Err(io_err(pp, e)),
                };
            rmtree(parent_fd.as_fd(), &name_c, path, opts)
        } else {
            rmtree(rustix::fs::CWD, &name_c, path, opts)
        }
    }

    fn rmtree(
        parent_fd: BorrowedFd<'_>,
        name: &CStr,
        path: &Path,
        opts: Options,
    ) -> Result<(), RumError> {
        let self_fd: OwnedFd = match rustix::fs::openat(
            parent_fd,
            name,
            OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(e) if opts.force && e == rustix::io::Errno::NOENT => return Ok(()),
            Err(e) => return Err(io_err(path, e)),
        };

        let mut files: Vec<CString> = Vec::new();
        let mut dirs: Vec<CString> = Vec::new();

        {
            let dir = Dir::read_from(&self_fd).map_err(|e| io_err(path, e))?;
            for entry in dir {
                let entry = entry.map_err(|e| io_err(path, e))?;
                let entry_name = entry.file_name();
                if is_dot_or_dotdot(entry_name) {
                    continue;
                }
                let owned = entry_name.to_owned();
                match entry.file_type() {
                    FileType::Directory => dirs.push(owned),
                    _ => files.push(owned),
                }
            }
        }

        let self_borrow = self_fd.as_fd();

        files
            .par_iter()
            .try_for_each(|entry_name| unlink_child(self_borrow, entry_name, path, opts, false))?;

        match dirs.as_slice() {
            [] => {}
            [only] => {
                let sub_path = path.join(Path::new(std::ffi::OsStr::from_bytes(only.as_bytes())));
                rmtree(self_borrow, only, &sub_path, opts)?;
            }
            _ => {
                dirs.par_iter().try_for_each(|entry_name| {
                    let sub_path = path.join(Path::new(std::ffi::OsStr::from_bytes(
                        entry_name.as_bytes(),
                    )));
                    rmtree(self_borrow, entry_name, &sub_path, opts)
                })?;
            }
        }

        drop(self_fd);

        match rustix::fs::unlinkat(parent_fd, name, AtFlags::REMOVEDIR) {
            Ok(()) => {
                log_removed_dir(path, opts);
                Ok(())
            }
            Err(e) if opts.force && e == rustix::io::Errno::NOENT => Ok(()),
            Err(e) => Err(io_err(path, e)),
        }
    }

    fn unlink_child(
        dir_fd: BorrowedFd<'_>,
        name: &CStr,
        parent_path: &Path,
        opts: Options,
        is_dir: bool,
    ) -> Result<(), RumError> {
        let flags = if is_dir {
            AtFlags::REMOVEDIR
        } else {
            AtFlags::empty()
        };
        match rustix::fs::unlinkat(dir_fd, name, flags) {
            Ok(()) => {
                if opts.verbose {
                    let child =
                        parent_path.join(Path::new(std::ffi::OsStr::from_bytes(name.to_bytes())));
                    if is_dir {
                        println!("removed directory '{}'", child.display());
                    } else {
                        println!("removed '{}'", child.display());
                    }
                }
                Ok(())
            }
            Err(e) if opts.force && e == rustix::io::Errno::NOENT => Ok(()),
            Err(e) => {
                let child =
                    parent_path.join(Path::new(std::ffi::OsStr::from_bytes(name.to_bytes())));
                Err(io_err(&child, e))
            }
        }
    }

    fn is_dot_or_dotdot(name: &CStr) -> bool {
        let bytes = name.to_bytes();
        matches!(bytes, b"." | b"..")
    }
}

#[cfg(not(unix))]
mod fallback {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use rayon::prelude::*;

    use super::{Options, RumError, log_removed_dir};

    pub fn remove_tree(dir: &Path, opts: Options) -> Result<(), RumError> {
        let entries = match fs::read_dir(dir) {
            Ok(it) => it,
            Err(e) if opts.force && e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => {
                return Err(RumError::Io {
                    path: dir.to_path_buf(),
                    source: e,
                });
            }
        };

        let mut files: Vec<PathBuf> = Vec::new();
        let mut dirs: Vec<PathBuf> = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| RumError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let ft = entry.file_type().map_err(|e| RumError::Io {
                path: entry.path(),
                source: e,
            })?;
            if ft.is_dir() {
                dirs.push(entry.path());
            } else {
                files.push(entry.path());
            }
        }

        files
            .par_iter()
            .try_for_each(|p| match fs::remove_file(p) {
                Ok(()) => {
                    if opts.verbose {
                        println!("removed '{}'", p.display());
                    }
                    Ok(())
                }
                Err(e) if opts.force && e.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(RumError::Io {
                    path: p.clone(),
                    source: e,
                }),
            })?;

        dirs.par_iter().try_for_each(|p| remove_tree(p, opts))?;

        match fs::remove_dir(dir) {
            Ok(()) => {
                log_removed_dir(dir, opts);
                Ok(())
            }
            Err(e) if opts.force && e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(RumError::Io {
                path: dir.to_path_buf(),
                source: e,
            }),
        }
    }
}

#[cfg(unix)]
fn remove_tree(path: &Path, opts: Options) -> Result<(), RumError> {
    unix_fast::remove_tree(path, opts)
}

#[cfg(not(unix))]
fn remove_tree(path: &Path, opts: Options) -> Result<(), RumError> {
    fallback::remove_tree(path, opts)
}
