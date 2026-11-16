use std::fmt::Write as _;
use std::fs;

use crate::langs::{
    compose, css, elixir, gleam, js, just, nix, nomad, nu, playwright, pnpm, rust, swift, tofu,
};
use crate::utils::output::Output;
use crate::{config, diagnostics, output, types};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Config(#[from] color_eyre::Report),
    #[error(transparent)]
    Diagnostics(#[from] diagnostics::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Format(#[from] std::fmt::Error),
}

#[derive(Debug, Clone, Copy, clap::Subcommand)]
pub enum Command {
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

pub fn run(command: Command, create_config: bool) -> Result<Output, Error> {
    let existing_config = config::load_config()?;

    let config = if create_config {
        types::Config::default()
    } else {
        existing_config.clone()
    };

    let commands_to_run: &[SingleCommand] = match command {
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
    let mut has_errors = false;
    let mut all_violations: Vec<types::Violation> = Vec::new();
    let mut text = String::new();

    for cmd in commands_to_run {
        let result = run_check(*cmd);
        diagnostics::append(&result.skipped)?;

        let filtered = config::filter_violations(result.violations, &config);
        total_files += result.file_count;
        all_violations.extend(filtered);

        if !result.errors.is_empty() {
            has_errors = true;
            for error in result.errors {
                writeln!(text, "Error: {error}")?;
            }
        }
    }

    if create_config {
        let toml_content = output::generate_config_toml(&all_violations, &existing_config);
        let action = if existing_config.ignore.is_some() {
            "Updated"
        } else {
            "Created"
        };
        fs::write("slop-detector.toml", toml_content)?;
        writeln!(
            text,
            "{action} slop-detector.toml with {} violations excluded",
            all_violations.len()
        )?;
        return Ok(Output {
            success: true,
            text,
        });
    }

    if !all_violations.is_empty() {
        let violations = output::format_violations(&all_violations);
        text.push_str(&violations);
        text.push('\n');
    }

    if has_errors || !all_violations.is_empty() {
        return Ok(Output {
            success: false,
            text,
        });
    }

    writeln!(text, "\u{2713} {total_files} files checked")?;
    Ok(Output {
        success: true,
        text,
    })
}
