use std::fs;

use hcl::Body;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[];
const PATTERNS: &[&str] = &["tofu/**/*.tf"];

struct BannedItem {
    hint: &'static str,
    type_name: &'static str,
}

const BANNED_DATA_SOURCES: &[BannedItem] = &[BannedItem {
    hint: "use a provider instead of external programs",
    type_name: "external",
}];

const BANNED_PROVIDERS: &[BannedItem] = &[BannedItem {
    hint: "enables running arbitrary programs",
    type_name: "external",
}];

const BANNED_PROVISIONERS: &[BannedItem] = &[
    BannedItem {
        hint: "use a provider instead",
        type_name: "local-exec",
    },
    BannedItem {
        hint: "use a provider instead",
        type_name: "remote-exec",
    },
];

const BANNED_RESOURCES: &[BannedItem] = &[BannedItem {
    hint: "use a proper resource type",
    type_name: "terraform_data",
}];

struct ParsedTfFile {
    body: Body,
    path: String,
}

struct Rule {
    check: fn(body: &Body, path: &str) -> Vec<Violation>,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_banned_resources,
    },
    Rule {
        check: check_banned_data_sources,
    },
    Rule {
        check: check_banned_provisioners,
    },
    Rule {
        check: check_banned_providers,
    },
];

fn check_banned_resources(body: &Body, path: &str) -> Vec<Violation> {
    body.blocks()
        .filter(|block| block.identifier.as_str() == "resource")
        .filter_map(|block| {
            let labels: Vec<_> = block.labels.iter().collect();
            if labels.len() < 2 {
                return None;
            }
            let resource_type = labels.first().map_or("", |label| label.as_str());
            let resource_name = labels.get(1).map_or("", |label| label.as_str());

            BANNED_RESOURCES
                .iter()
                .find(|banned| resource_type == banned.type_name)
                .map(|banned| Violation {
                    errors: vec![format!("resource \"{resource_type}\" \"{resource_name}\"")],
                    file: path.to_string(),
                    hint: Some(banned.hint.to_string()),
                    rule: "banned-resource".to_string(),
                })
        })
        .collect()
}

fn check_banned_data_sources(body: &Body, path: &str) -> Vec<Violation> {
    body.blocks()
        .filter(|block| block.identifier.as_str() == "data")
        .filter_map(|block| {
            let labels: Vec<_> = block.labels.iter().collect();
            if labels.len() < 2 {
                return None;
            }
            let data_type = labels.first().map_or("", |label| label.as_str());
            let data_name = labels.get(1).map_or("", |label| label.as_str());

            BANNED_DATA_SOURCES
                .iter()
                .find(|banned| data_type == banned.type_name)
                .map(|banned| Violation {
                    errors: vec![format!("data \"{data_type}\" \"{data_name}\"")],
                    file: path.to_string(),
                    hint: Some(banned.hint.to_string()),
                    rule: "banned-data-source".to_string(),
                })
        })
        .collect()
}

fn check_banned_provisioners(body: &Body, path: &str) -> Vec<Violation> {
    body.blocks()
        .filter(|block| block.identifier.as_str() == "resource")
        .flat_map(|block| {
            let labels: Vec<_> = block.labels.iter().collect();
            let resource_type = labels.first().map_or("", |label| label.as_str());
            let resource_name = labels.get(1).map_or("", |label| label.as_str());

            block
                .body
                .blocks()
                .filter(|inner| inner.identifier.as_str() == "provisioner")
                .filter_map(|inner| {
                    let prov_labels: Vec<_> = inner.labels.iter().collect();
                    let prov_type = prov_labels.first()?.as_str();

                    BANNED_PROVISIONERS
                        .iter()
                        .find(|banned| prov_type == banned.type_name)
                        .map(|banned| Violation {
                            errors: vec![format!(
                                "resource \"{resource_type}\" \"{resource_name}\" provisioner \"{prov_type}\""
                            )],
                            file: path.to_string(),
                            hint: Some(banned.hint.to_string()),
                            rule: "banned-provisioner".to_string(),
                        })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn check_banned_providers(body: &Body, path: &str) -> Vec<Violation> {
    body.blocks()
        .filter(|block| block.identifier.as_str() == "terraform")
        .flat_map(|block| {
            block
                .body
                .blocks()
                .filter(|inner| inner.identifier.as_str() == "required_providers")
                .flat_map(|inner| {
                    inner.body.attributes().filter_map(|attr| {
                        let provider_name = attr.key.as_str();

                        BANNED_PROVIDERS
                            .iter()
                            .find(|banned| provider_name == banned.type_name)
                            .map(|banned| Violation {
                                errors: vec![format!("provider \"{provider_name}\"")],
                                file: path.to_string(),
                                hint: Some(banned.hint.to_string()),
                                rule: "banned-provider".to_string(),
                            })
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn find(patterns: &[&str], excludes: &[&str]) -> Vec<String> {
    patterns
        .iter()
        .flat_map(|pattern| find_files(pattern, excludes).unwrap_or_default())
        .collect()
}

fn parse(paths: Vec<String>) -> (Vec<ParsedTfFile>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match hcl::from_str(&content) {
                Ok(body) => parsed.push(ParsedTfFile { body, path }),
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

fn run_rules(files: &[ParsedTfFile], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules
                .iter()
                .flat_map(|rule| (rule.check)(&file.body, &file.path))
        })
        .collect()
}

pub fn check() -> CheckResult {
    let paths = find(PATTERNS, EXCLUDES);
    let (parsed, errors) = parse(paths);
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

    fn parse_fixture(name: &str) -> Body {
        let path = format!("fixtures/tofu/{name}.tf");
        let content = fs::read_to_string(&path).expect("fixture should exist");
        hcl::from_str(&content).expect("valid hcl")
    }

    #[test]
    fn banned_resource_detects_violation() {
        let body = parse_fixture("banned-resource");
        let violations = check_banned_resources(&body, "test");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn banned_resource_allows_valid() {
        let body = parse_fixture("valid-resource");
        let violations = check_banned_resources(&body, "test");
        assert!(violations.is_empty());
    }

    #[test]
    fn banned_data_source_detects_violation() {
        let body = parse_fixture("banned-data-source");
        let violations = check_banned_data_sources(&body, "test");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn banned_data_source_allows_valid() {
        let body = parse_fixture("valid-data");
        let violations = check_banned_data_sources(&body, "test");
        assert!(violations.is_empty());
    }

    #[test]
    fn banned_provisioner_detects_violation() {
        let body = parse_fixture("banned-provisioner");
        let violations = check_banned_provisioners(&body, "test");
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn banned_provider_detects_violation() {
        let body = parse_fixture("banned-provider");
        let violations = check_banned_providers(&body, "test");
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn banned_provider_allows_valid() {
        let body = parse_fixture("valid-provider");
        let violations = check_banned_providers(&body, "test");
        assert!(violations.is_empty());
    }
}
