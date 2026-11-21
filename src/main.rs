mod commands;
mod config;
mod diagnostics;
mod files;
mod langs;
mod output;
mod types;
mod utils;

use std::process::ExitCode;

use clap::Parser as _;

use commands::{check, version};

#[derive(Debug, clap::Parser)]
#[command(name = "slop-detector")]
#[command(version = version::text())]
#[command(
    about = "Linter for detecting anti-patterns in Compose, CSS, Elixir, Gleam, JS/TS, Just, Nix, Nomad, Nushell, Playwright, pnpm, Rust, Swift, and OpenTofu code"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(long = "create-config-for-violations", global = true)]
    create_config: bool,
}

#[derive(Debug, clap::Subcommand)]
enum Command {
    #[command(flatten)]
    Check(check::Command),
    #[command(about = "Show version and build metadata")]
    Version,
}

fn main() -> color_eyre::Result<ExitCode> {
    color_eyre::install()?;
    let args = Args::parse();
    let output = match args.command {
        Some(Command::Version) => {
            let result = version::run();
            Ok(result)
        }
        Some(Command::Check(command)) => check::run(command, args.create_config),
        None => check::run(check::Command::All, args.create_config),
    };
    let output = match output {
        Ok(result) => result,
        Err(error) => {
            let event = check::failure_event(&error);
            if let Err(log_error) = diagnostics::append(&[event]) {
                eprintln!("Warning: could not write error log: {log_error}");
            }
            let report = error.into();
            return Err(report);
        }
    };
    print!("{}", output.text);
    if output.success {
        return Ok(ExitCode::SUCCESS);
    }
    Ok(ExitCode::FAILURE)
}
