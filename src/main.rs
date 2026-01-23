mod config;
mod files;
mod langs;
mod output;
mod types;

use std::fs;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use color_eyre::Result;

use config::{filter_violations, load_config};
use langs::{gleam, js, rust, tofu};
use output::{format_violations, generate_config_toml};
use types::Violation;

#[derive(Debug, Clone, ValueEnum)]
enum Command {
    All,
    Gleam,
    Js,
    Rust,
    Tofu,
}

#[derive(Debug, Clone, Copy)]
enum SingleCommand {
    Gleam,
    Js,
    Rust,
    Tofu,
}

const ALL_COMMANDS: &[SingleCommand] = &[
    SingleCommand::Gleam,
    SingleCommand::Js,
    SingleCommand::Rust,
    SingleCommand::Tofu,
];

fn run_check(cmd: SingleCommand) -> types::CheckResult {
    match cmd {
        SingleCommand::Gleam => gleam::check(),
        SingleCommand::Js => js::check(),
        SingleCommand::Rust => rust::check(),
        SingleCommand::Tofu => tofu::check(),
    }
}

#[derive(Parser, Debug)]
#[command(name = "slop-detector")]
#[command(about = "Linter for detecting anti-patterns in Gleam, JS/TS, Rust, and OpenTofu code")]
struct Args {
    #[arg(value_enum, default_value = "all")]
    command: Command,

    #[arg(long = "create-config-for-violations")]
    create_config: bool,
}

fn main() -> Result<ExitCode> {
    color_eyre::install()?;

    let args = Args::parse();

    let config = if args.create_config {
        types::Config::default()
    } else {
        load_config()?
    };

    let commands_to_run: &[SingleCommand] = match args.command {
        Command::All => ALL_COMMANDS,
        Command::Gleam => &[SingleCommand::Gleam],
        Command::Js => &[SingleCommand::Js],
        Command::Rust => &[SingleCommand::Rust],
        Command::Tofu => &[SingleCommand::Tofu],
    };

    let mut total_files: usize = 0;
    let mut has_errors = false;
    let mut all_violations: Vec<Violation> = Vec::new();

    for cmd in commands_to_run {
        let result = run_check(*cmd);

        let filtered = filter_violations(result.violations, &config);
        total_files += result.file_count;
        all_violations.extend(filtered);

        if !result.errors.is_empty() {
            has_errors = true;
            for error in result.errors {
                println!("Error: {error}");
            }
        }
    }

    if args.create_config {
        let toml_content = generate_config_toml(&all_violations);
        fs::write("slop-detector.toml", toml_content)?;
        println!(
            "Created slop-detector.toml with {} violations excluded",
            all_violations.len()
        );
        return Ok(ExitCode::SUCCESS);
    }

    if !all_violations.is_empty() {
        println!("{}", format_violations(&all_violations));
    }

    if has_errors || !all_violations.is_empty() {
        return Ok(ExitCode::FAILURE);
    }

    println!("\u{2713} {total_files} files checked");
    Ok(ExitCode::SUCCESS)
}
