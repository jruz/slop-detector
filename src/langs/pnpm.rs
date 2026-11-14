use std::collections::BTreeMap;
use std::fs;

use crate::files;
use crate::types::{AppError, CheckResult, Violation};

type Settings = BTreeMap<String, serde_json::Value>;

const PATTERNS: &[&str] = &["pnpm-workspace.yaml"];

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Read(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] serde_saphyr::Error),
}

struct Rule {
    check: fn(&Settings) -> Vec<String>,
    hint: &'static str,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_minimum_release_age_exclude,
        hint: "remove exclusions instead of bypassing the release-age policy",
        name: "no-minimum-release-age-exclude",
    },
    Rule {
        check: check_minimum_release_age,
        hint: "set minimumReleaseAge to a positive number of minutes",
        name: "minimum-release-age",
    },
];

fn check_no_minimum_release_age_exclude(settings: &Settings) -> Vec<String> {
    match settings.get("minimumReleaseAgeExclude") {
        None => vec![],
        Some(serde_json::Value::Array(entries)) => entries
            .iter()
            .map(|entry| {
                format!("minimumReleaseAgeExclude: {entry} bypasses the release-age policy")
            })
            .collect(),
        Some(_) => vec!["minimumReleaseAgeExclude must be an empty list or absent".to_string()],
    }
}

fn check_minimum_release_age(settings: &Settings) -> Vec<String> {
    let zero: f64 = 0.0;
    if settings
        .get("minimumReleaseAge")
        .and_then(serde_json::Value::as_f64)
        .is_some_and(|age| age > zero)
    {
        return vec![];
    }
    vec!["minimumReleaseAge must be explicitly set to a number greater than zero".to_string()]
}

fn parse(path: &str) -> Result<Settings, Error> {
    let source = fs::read_to_string(path)?;
    let settings = serde_saphyr::from_str(&source)?;
    Ok(settings)
}

fn run_rules(path: &str, settings: &Settings) -> Vec<Violation> {
    RULES
        .iter()
        .filter_map(|rule| {
            let errors = (rule.check)(settings);
            if errors.is_empty() {
                return None;
            }
            let hint = rule.hint.to_string();
            Some(Violation {
                errors,
                file: path.to_string(),
                hint: Some(hint),
                rule: rule.name.to_string(),
            })
        })
        .collect()
}

fn app_error(file: String, error: Error) -> AppError {
    match error {
        Error::Read(error) => AppError::Read {
            file,
            message: error.to_string(),
        },
        Error::Parse(error) => AppError::Parse {
            file,
            message: error.to_string(),
        },
    }
}

pub fn check() -> CheckResult {
    let mut result = CheckResult {
        skipped: Vec::new(),
        errors: vec![],
        file_count: 0,
        violations: vec![],
    };
    let paths = match files::find_files(PATTERNS, &[]) {
        Ok(paths) => paths,
        Err(error) => {
            result.errors.push(AppError::Glob {
                pattern: PATTERNS.join(", "),
                message: error.to_string(),
            });
            return result;
        }
    };
    for path in paths {
        match parse(&path) {
            Ok(settings) => {
                result.file_count += 1;
                let violations = run_rules(&path, &settings);
                result.violations.extend(violations);
            }
            Err(error) => {
                let error = app_error(path, error);
                result.errors.push(error);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        check_minimum_release_age, check_no_minimum_release_age_exclude, parse, run_rules,
    };

    #[test]
    fn no_minimum_release_age_exclude_detects_violation() {
        let settings = parse("fixtures/pnpm/no-minimum-release-age-exclude.yaml")
            .expect("fixture should parse");
        let errors = check_no_minimum_release_age_exclude(&settings);
        assert_eq!(errors.len(), 4);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("@vercel/oidc@4.0.3"))
        );
    }

    #[test]
    fn no_minimum_release_age_exclude_allows_valid() {
        let settings = parse("fixtures/pnpm/valid-no-minimum-release-age-exclude.yaml")
            .expect("fixture should parse");
        assert_eq!(
            check_no_minimum_release_age_exclude(&settings),
            Vec::<String>::new()
        );
    }

    #[test]
    fn no_minimum_release_age_exclude_handles_yaml_forms() {
        for source in [
            "minimumReleaseAgeExclude: [eslint@10.12.0]",
            "'minimumReleaseAgeExclude': [eslint@10.12.0]",
            "exclusions: &exclusions [eslint@10.12.0]\nminimumReleaseAgeExclude: *exclusions",
        ] {
            let settings = serde_saphyr::from_str(source).expect("settings should parse");
            assert_eq!(check_no_minimum_release_age_exclude(&settings).len(), 1);
        }
    }

    #[test]
    fn no_minimum_release_age_exclude_rejects_invalid_values() {
        for value in ["null", "false", "{}", "eslint"] {
            let source = format!("minimumReleaseAgeExclude: {value}");
            let settings = serde_saphyr::from_str(&source).expect("settings should parse");
            assert_eq!(check_no_minimum_release_age_exclude(&settings).len(), 1);
        }
    }

    #[test]
    fn minimum_release_age_detects_violation() {
        let settings =
            parse("fixtures/pnpm/minimum-release-age.yaml").expect("fixture should parse");
        assert_eq!(check_minimum_release_age(&settings).len(), 1);
    }

    #[test]
    fn minimum_release_age_allows_valid() {
        let settings =
            parse("fixtures/pnpm/valid-minimum-release-age.yaml").expect("fixture should parse");
        assert_eq!(check_minimum_release_age(&settings), Vec::<String>::new());
        assert_eq!(
            check_no_minimum_release_age_exclude(&settings),
            Vec::<String>::new()
        );
    }

    #[test]
    fn minimum_release_age_requires_explicit_positive_number() {
        for source in [
            "packages: ['clients/*']",
            "minimumReleaseAge: -1",
            "minimumReleaseAge: null",
            "minimumReleaseAge: false",
            "minimumReleaseAge: '10080'",
        ] {
            let settings = serde_saphyr::from_str(source).expect("settings should parse");
            assert_eq!(check_minimum_release_age(&settings).len(), 1);
        }
    }

    #[test]
    fn minimum_release_age_allows_positive_numbers() {
        for source in ["minimumReleaseAge: 1", "minimumReleaseAge: 0.5"] {
            let settings = serde_saphyr::from_str(source).expect("settings should parse");
            assert_eq!(check_minimum_release_age(&settings), Vec::<String>::new());
        }
    }

    #[test]
    fn run_rules_preserves_nested_path() {
        let settings =
            serde_saphyr::from_str("minimumReleaseAge: 0").expect("settings should parse");
        let violations = run_rules("./clients/web/pnpm-workspace.yaml", &settings);
        assert_eq!(violations.len(), 1);
        assert!(violations.iter().all(|violation| {
            violation.file == "./clients/web/pnpm-workspace.yaml"
                && violation.rule == "minimum-release-age"
        }));
    }

    #[test]
    fn malformed_yaml_fails_parsing() {
        serde_saphyr::from_str::<super::Settings>("minimumReleaseAge: [")
            .expect_err("malformed YAML must fail");
    }
}
