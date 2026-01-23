use std::fs;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[".git", "build", "fixtures"];
const GLEAM_TOML_PATTERNS: &[&str] = &["**/gleam.toml"];

struct GleamTomlRule {
    check: fn(toml: &toml::Value, path: &str) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const GLEAM_TOML_RULES: &[GleamTomlRule] = &[GleamTomlRule {
    check: check_exact_versions,
    hint: None,
    name: "exact-versions",
}];

struct ParsedGleamToml {
    path: String,
    toml: toml::Value,
}

fn is_unpinned_version(version: &str) -> bool {
    let trimmed = version.trim();
    trimmed.starts_with(">=")
        || trimmed.starts_with('>')
        || trimmed.starts_with("<=")
        || trimmed.starts_with('<')
        || trimmed.starts_with("~>")
        || trimmed.contains(" and ")
        || trimmed.contains(" or ")
}

fn check_exact_versions(toml: &toml::Value, _path: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let dep_sections = ["dependencies", "dev-dependencies"];

    for section in dep_sections {
        let Some(deps) = toml.get(section).and_then(toml::Value::as_table) else {
            continue;
        };

        for (name, value) in deps {
            let version = match value {
                toml::Value::String(ver) => Some(ver.as_str()),
                toml::Value::Table(table) => table.get("version").and_then(toml::Value::as_str),
                _ => None,
            };

            if let Some(ver) = version
                && is_unpinned_version(ver)
            {
                errors.push(format!("{section}.{name}: \"{ver}\" not pinned"));
            }
        }
    }

    errors
}

fn find(patterns: &[&str], excludes: &[&str]) -> Vec<String> {
    patterns
        .iter()
        .flat_map(|pattern| find_files(pattern, excludes).unwrap_or_default())
        .collect()
}

fn parse_gleam_tomls(paths: Vec<String>) -> (Vec<ParsedGleamToml>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<toml::Value>(&content) {
                Ok(toml) => parsed.push(ParsedGleamToml { path, toml }),
                Err(err) => errors.push(AppError::ParseFailed {
                    file: path,
                    message: err.to_string(),
                }),
            },
            Err(err) => errors.push(AppError::ReadFailed {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_gleam_toml_rules(files: &[ParsedGleamToml], rules: &[GleamTomlRule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.toml, &file.path);
                if errors.is_empty() {
                    return None;
                }
                Some(Violation {
                    errors,
                    file: file.path.clone(),
                    hint: rule.hint.map(String::from),
                    rule: rule.name.to_string(),
                })
            })
        })
        .collect()
}

pub fn check() -> CheckResult {
    let gleam_toml_paths = find(GLEAM_TOML_PATTERNS, EXCLUDES);

    let (parsed_tomls, errors) = parse_gleam_tomls(gleam_toml_paths);
    let violations = run_gleam_toml_rules(&parsed_tomls, GLEAM_TOML_RULES);

    CheckResult {
        errors,
        file_count: parsed_tomls.len(),
        violations,
    }
}
