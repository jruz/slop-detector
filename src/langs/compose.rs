use std::fs;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[];
const PATTERNS: &[&str] = &["compose.*.yaml"];

struct Rule {
    check: fn(&str) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_env_passthrough,
        hint: Some("don't redeclare vars --env-file passes them automatically to all processes"),
        name: "no-env-passthrough",
    },
    Rule {
        check: check_no_latest_tag,
        hint: Some("pin to an exact version tag"),
        name: "no-latest-tag",
    },
];

fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn check_no_env_passthrough(source: &str) -> Vec<String> {
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let trimmed = line.trim();
            let (key, value) = trimmed.split_once(':')?;
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            let expected = format!("${{{key}}}");
            if value != expected {
                return None;
            }
            if key.ends_with("_ARGS") {
                return None;
            }
            Some(format!("line {}: {key} is a passthrough", idx + 1))
        })
        .collect()
}

fn check_no_latest_tag(source: &str) -> Vec<String> {
    source
        .match_indices("image:")
        .filter_map(|(offset, _)| {
            let rest = source.get(offset + "image:".len()..)?;
            let image = rest.lines().next()?.trim().trim_matches('"');
            if image.is_empty() {
                return None;
            }
            let after_registry = image.rsplit_once('/').map_or(image, |(_, name)| name);
            let has_pinned_tag = after_registry
                .split_once(':')
                .is_some_and(|(_, tag)| tag != "latest");
            if has_pinned_tag {
                return None;
            }
            let line = line_number(source, offset);
            Some(format!("line {line}: {image} is not pinned"))
        })
        .collect()
}

struct ParsedCompose {
    path: String,
    source: String,
}

fn find(patterns: &[&str], excludes: &[&str]) -> (Vec<String>, Vec<AppError>) {
    match find_files(patterns, excludes) {
        Ok(found) => (found, vec![]),
        Err(err) => (
            vec![],
            vec![AppError::Glob {
                pattern: patterns.join(", "),
                message: err.to_string(),
            }],
        ),
    }
}

fn parse(paths: Vec<String>) -> (Vec<ParsedCompose>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => parsed.push(ParsedCompose { path, source }),
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_rules(files: &[ParsedCompose], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source);
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
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let (parsed, read_errors) = parse(paths);
    errors.extend(read_errors);
    let violations = run_rules(&parsed, RULES);

    CheckResult {
        errors,
        file_count: parsed.len(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("fixtures/compose/{name}.yaml");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    #[test]
    fn no_env_passthrough_detects_violation() {
        let source = read_fixture("no-env-passthrough");
        let errors = check_no_env_passthrough(&source);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_env_passthrough_allows_valid() {
        let source = read_fixture("valid-no-env-passthrough");
        let errors = check_no_env_passthrough(&source);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_latest_tag_detects_violation() {
        let source = read_fixture("no-latest-tag");
        let errors = check_no_latest_tag(&source);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_latest_tag_allows_valid() {
        let source = read_fixture("valid-no-latest-tag");
        let errors = check_no_latest_tag(&source);
        assert_eq!(errors, Vec::<String>::new());
    }
}
