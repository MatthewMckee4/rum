use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use rum::{Options, remove_paths};

/// A fast `rm` replacement.
#[derive(Debug, Parser)]
#[command(name = "rum", version, about)]
struct Cli {
    /// Remove directories and their contents recursively.
    #[arg(short = 'r', short_alias = 'R', long)]
    recursive: bool,

    /// Ignore nonexistent files and arguments, never prompt.
    #[arg(short, long)]
    force: bool,

    /// Explain what is being done.
    #[arg(short, long)]
    verbose: bool,

    /// Files or directories to remove.
    #[arg(required_unless_present = "force", num_args = 0..)]
    paths: Vec<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.paths.is_empty() {
        return ExitCode::SUCCESS;
    }

    let opts = Options {
        recursive: cli.recursive,
        force: cli.force,
        verbose: cli.verbose,
    };

    let errors = remove_paths(&cli.paths, opts);
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        for e in &errors {
            eprintln!("rum: {e}");
        }
        ExitCode::from(1)
    }
}
