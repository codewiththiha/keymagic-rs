//! Command-line tools for KMS compilation, KM2 inspection and corpus replay.

mod compile;
mod install;
mod json;
mod repl;
mod replay;
mod resolver;
mod style;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use std::path::PathBuf;
use std::process::ExitCode;

fn cli() -> Command {
    Command::new("keymagic")
        .version(env!("CARGO_PKG_VERSION"))
        .about("`KeyMagic` keyboard tools.")
        .arg(
            Arg::new("json")
                .long("json")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Print results as JSON"),
        )
        .arg(
            Arg::new("color")
                .long("color")
                .global(true)
                .value_parser(["auto", "always", "never"])
                .default_value("auto")
                .help("Colour output: auto (terminal only), always or never"),
        )
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("compile")
                .about("Compile a KMS file to KM2.")
                .arg(path_arg("input", "KMS source file"))
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_parser(value_parser!(PathBuf))
                        .help("Output path; defaults to the input with a .km2 extension"),
                ),
        )
        .subcommand(
            Command::new("check")
                .about("Report diagnostics for a KMS file.")
                .arg(path_arg("input", "KMS source file")),
        )
        .subcommand(
            Command::new("fmt")
                .about("Print, rewrite, or check the canonical form of a KMS file.")
                .arg(path_arg("input", "KMS source file"))
                .arg(
                    Arg::new("write")
                        .long("write")
                        .action(ArgAction::SetTrue)
                        .conflicts_with("check")
                        .help("Rewrite the file in place"),
                )
                .arg(
                    Arg::new("check")
                        .long("check")
                        .action(ArgAction::SetTrue)
                        .help("Exit with status 1 when the file is not canonical"),
                ),
        )
        .subcommand(
            Command::new("dump")
                .about("Summarize a KM2 file.")
                .arg(path_arg("input", "KM2 file")),
        )
        .subcommand(
            Command::new("test")
                .about("Replay corpus cases against the engine.")
                .arg(path_arg("cases", "Directory of case files (*.toml)")),
        )
        .subcommand(
            Command::new("repl")
                .about("Type characters from stdin, one line at a time.")
                .arg(path_arg("keyboard", "KM2 or KMS keyboard")),
        )
        .subcommand(Command::new("doctor").about("Check the local setup (not implemented yet)."))
        .subcommand(
            Command::new("install")
                .about("Write the IBus component file for this user.")
                .arg(
                    Arg::new("user")
                        .long("user")
                        .action(ArgAction::SetTrue)
                        .required(true)
                        .help("Install for the current user only"),
                ),
        )
}

fn path_arg(id: &'static str, help: &'static str) -> Arg {
    Arg::new(id)
        .value_parser(value_parser!(PathBuf))
        .required(true)
        .help(help)
}

fn path(matches: &ArgMatches, id: &str) -> PathBuf {
    matches.get_one::<PathBuf>(id).cloned().unwrap_or_default()
}

fn main() -> ExitCode {
    let matches = cli().get_matches();
    let json = matches.get_flag("json");
    style::choose(
        matches
            .get_one::<String>("color")
            .map_or("auto", String::as_str),
    );
    let result = match matches.subcommand() {
        Some(("compile", args)) => compile::compile(
            &path(args, "input"),
            args.get_one::<PathBuf>("output").map(PathBuf::as_path),
            json,
        ),
        Some(("check", args)) => compile::check(&path(args, "input"), json),
        Some(("fmt", args)) => compile::fmt(
            &path(args, "input"),
            args.get_flag("write"),
            args.get_flag("check"),
            json,
        ),
        Some(("dump", args)) => compile::dump(&path(args, "input"), json),
        Some(("test", args)) => replay::run(&path(args, "cases"), json),
        Some(("repl", args)) => repl::run(&path(args, "keyboard"), json),
        Some(("doctor", _)) => Err("doctor is not implemented yet".to_owned()),
        Some(("install", _)) => install::user(),
        _ => Err("no command given".to_owned()),
    };
    result.unwrap_or_else(|message| {
        eprintln!("error: {message}");
        ExitCode::from(2)
    })
}
