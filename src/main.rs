mod config;
mod files;
mod langs;
mod output;
mod types;
mod version;

use std::fs;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use color_eyre::Result;

use config::{filter_violations, load_config};
use langs::{
    compose, css, elixir, gleam, js, just, nix, nomad, nu, playwright, pnpm, rust, swift, tofu,
};
use output::{format_violations, generate_config_toml};
use types::Violation;

#[derive(Debug, Clone, ValueEnum)]
enum Command {
    All,
    Compose,
    Css,
    Elixir,
    Gleam,
    Js,
    Just,
    Nix,
    Nomad,
    Nu,
    Playwright,
    Pnpm,
    Rust,
    Swift,
    Tofu,
}

#[derive(Debug, Clone, Copy)]
enum SingleCommand {
    Compose,
    Css,
    Elixir,
    Gleam,
    Js,
    Just,
    Nix,
    Nomad,
    Nu,
    Playwright,
    Pnpm,
    Rust,
    Swift,
    Tofu,
}

const ALL_COMMANDS: &[SingleCommand] = &[
    SingleCommand::Compose,
    SingleCommand::Css,
    SingleCommand::Elixir,
    SingleCommand::Gleam,
    SingleCommand::Js,
    SingleCommand::Just,
    SingleCommand::Nix,
    SingleCommand::Nomad,
    SingleCommand::Nu,
    SingleCommand::Playwright,
    SingleCommand::Pnpm,
    SingleCommand::Rust,
    SingleCommand::Swift,
    SingleCommand::Tofu,
];

fn run_check(cmd: SingleCommand) -> types::CheckResult {
    match cmd {
        SingleCommand::Compose => compose::check(),
        SingleCommand::Css => css::check(),
        SingleCommand::Elixir => elixir::check(),
        SingleCommand::Gleam => gleam::check(),
        SingleCommand::Js => js::check(),
        SingleCommand::Just => just::check(),
        SingleCommand::Nix => nix::check(),
        SingleCommand::Nomad => nomad::check(),
        SingleCommand::Nu => nu::check(),
        SingleCommand::Playwright => playwright::check(),
        SingleCommand::Pnpm => pnpm::check(),
        SingleCommand::Rust => rust::check(),
        SingleCommand::Swift => swift::check(),
        SingleCommand::Tofu => tofu::check(),
    }
}

#[derive(Parser, Debug)]
#[command(name = "slop-detector")]
#[command(version = version::clap_version())]
#[command(
    about = "Linter for detecting anti-patterns in Compose, CSS, Elixir, Gleam, JS/TS, Just, Nix, Nomad, Nushell, Playwright, pnpm, Rust, Swift, and OpenTofu code"
)]
struct Args {
    #[arg(value_enum, default_value = "all")]
    command: Command,

    #[arg(long = "create-config-for-violations")]
    create_config: bool,
}

fn skip_warning(skipped: &types::SkippedAnalysis) -> String {
    let target = match &skipped.recipe {
        Some(recipe) => format!("recipe {recipe} from {}", skipped.file),
        None => format!("file {}", skipped.file),
    };
    let reason = if skipped.reason.contains("Module not found")
        || skipped.reason.contains("File not found")
    {
        "import could not be resolved statically".to_string()
    } else {
        skipped
            .reason
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    format!("Warning: [nushell] skipped {target}: {reason}")
}

fn main() -> Result<ExitCode> {
    color_eyre::install()?;

    let args = Args::parse();

    let existing_config = load_config()?;

    let config = if args.create_config {
        types::Config::default()
    } else {
        existing_config.clone()
    };

    let commands_to_run: &[SingleCommand] = match args.command {
        Command::All => ALL_COMMANDS,
        Command::Compose => &[SingleCommand::Compose],
        Command::Css => &[SingleCommand::Css],
        Command::Elixir => &[SingleCommand::Elixir],
        Command::Gleam => &[SingleCommand::Gleam],
        Command::Js => &[SingleCommand::Js],
        Command::Just => &[SingleCommand::Just],
        Command::Nix => &[SingleCommand::Nix],
        Command::Nomad => &[SingleCommand::Nomad],
        Command::Nu => &[SingleCommand::Nu],
        Command::Playwright => &[SingleCommand::Playwright],
        Command::Pnpm => &[SingleCommand::Pnpm],
        Command::Rust => &[SingleCommand::Rust],
        Command::Swift => &[SingleCommand::Swift],
        Command::Tofu => &[SingleCommand::Tofu],
    };

    let mut total_files: usize = 0;
    let mut total_skipped: usize = 0;
    let mut has_errors = false;
    let mut all_violations: Vec<Violation> = Vec::new();

    for cmd in commands_to_run {
        let result = run_check(*cmd);
        total_skipped += result.skipped.len();
        for skipped in result.skipped {
            let warning = skip_warning(&skipped);
            println!("{warning}");
        }

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
        let toml_content = generate_config_toml(&all_violations, &existing_config);
        let action = if existing_config.ignore.is_some() {
            "Updated"
        } else {
            "Created"
        };
        fs::write("slop-detector.toml", toml_content)?;
        println!(
            "{action} slop-detector.toml with {} violations excluded",
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

    if total_skipped > 0 {
        println!("Checked files: {total_files}; skipped analyses: {total_skipped}");
        return Ok(ExitCode::SUCCESS);
    }

    println!("\u{2713} {total_files} files checked");
    Ok(ExitCode::SUCCESS)
}
