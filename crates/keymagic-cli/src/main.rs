//! Command-line tools for KMS compilation, KM2 inspection and corpus replay.

mod compile;
mod repl;
mod replay;
mod resolver;
mod style;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// KeyMagic keyboard tools.
#[derive(Parser)]
#[command(name = "keymagic", version)]
struct Cli {
    /// Print results as JSON.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile a KMS file to KM2.
    Compile {
        /// KMS source file.
        input: PathBuf,
        /// Output path; defaults to the input with a .km2 extension.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Report diagnostics for a KMS file.
    Check {
        /// KMS source file.
        input: PathBuf,
    },
    /// Print the canonical form of a KMS file.
    Fmt {
        /// KMS source file.
        input: PathBuf,
        /// Rewrite the file in place.
        #[arg(long, conflicts_with = "check")]
        write: bool,
        /// Exit with status 1 when the file is not canonical.
        #[arg(long)]
        check: bool,
    },
    /// Summarize a KM2 file.
    Dump {
        /// KM2 file.
        input: PathBuf,
    },
    /// Replay corpus cases against the engine.
    Test {
        /// Directory of case files (`*.toml`).
        cases: PathBuf,
    },
    /// Type characters from stdin, one line at a time.
    Repl {
        /// KM2 or KMS keyboard.
        keyboard: PathBuf,
    },
    /// Check the local setup (not implemented yet).
    Doctor,
    /// Install a keyboard (not implemented yet).
    Install,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Compile { input, output } => compile::compile(&input, output.as_deref(), cli.json),
        Command::Check { input } => compile::check(&input, cli.json),
        Command::Fmt {
            input,
            write,
            check,
        } => compile::fmt(&input, write, check, cli.json),
        Command::Dump { input } => compile::dump(&input, cli.json),
        Command::Test { cases } => replay::run(&cases, cli.json),
        Command::Repl { keyboard } => repl::run(&keyboard, cli.json),
        Command::Doctor => Err("doctor is not implemented yet".to_owned()),
        Command::Install => Err("install is not implemented yet".to_owned()),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::from(2)
    })
}
