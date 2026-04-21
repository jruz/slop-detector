use std::fs;

use rnix::Root;
use rnix::ast::{self, HasEntry as _};
use rowan::ast::AstNode as _;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &[];
const PATTERNS: &[&str] = &["*.nix"];

struct Rule {
    check: fn(source: &str, root: &Root) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_unstable_nixpkgs,
        hint: Some(
            "pin nixpkgs to a release branch like nixos-25.11 -- unstable is fine as a separate input named unstable",
        ),
        name: "no-unstable-nixpkgs",
    },
    Rule {
        check: check_no_manual_for_all_systems,
        hint: Some("use flake-utils instead of manually defining forAllSystems with genAttrs"),
        name: "no-manual-for-all-systems",
    },
];

fn line_number(source: &str, byte_offset: usize) -> usize {
    source.get(..byte_offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn attrpath_name(attrpath: &ast::Attrpath) -> Option<String> {
    let attrs: Vec<_> = attrpath.attrs().collect();
    let first = attrs.first()?;
    let text = first.syntax().text();
    let name = text.to_string();
    Some(name)
}

fn string_value(expr: &ast::Expr) -> Option<String> {
    let ast::Expr::Str(str_node) = expr else {
        return None;
    };
    let parts: Vec<_> = str_node.normalized_parts();
    if parts.len() != 1 {
        return None;
    }
    match parts.into_iter().next()? {
        ast::InterpolPart::Literal(lit) => Some(lit),
        ast::InterpolPart::Interpolation(_) => None,
    }
}

fn find_inputs_attrset(root: &Root) -> Option<ast::AttrSet> {
    let expr = root.expr()?;
    let attrset = match expr {
        ast::Expr::AttrSet(set) => set,
        ast::Expr::Lambda(lambda) => {
            let body = lambda.body()?;
            match body {
                ast::Expr::AttrSet(set) => set,
                _ => return None,
            }
        }
        _ => return None,
    };

    attrset.attrpath_values().find_map(|entry| {
        let attrpath = entry.attrpath()?;
        let name = attrpath_name(&attrpath)?;
        if name != "inputs" {
            return None;
        }
        match entry.value()? {
            ast::Expr::AttrSet(inner) => Some(inner),
            _ => None,
        }
    })
}

fn check_no_unstable_nixpkgs(source: &str, root: &Root) -> Vec<String> {
    let Some(inputs) = find_inputs_attrset(root) else {
        return vec![];
    };

    inputs
        .attrpath_values()
        .filter_map(|entry| {
            let attrpath = entry.attrpath()?;
            let input_name = attrpath_name(&attrpath)?;
            if input_name != "nixpkgs" {
                return None;
            }

            let value = entry.value()?;
            let url = extract_url_from_input(&value)?;
            if !url.contains("nixpkgs-unstable") {
                return None;
            }

            let offset: usize = entry.syntax().text_range().start().into();
            let line = line_number(source, offset);
            Some(format!("line {line}: nixpkgs input uses nixpkgs-unstable"))
        })
        .collect()
}

fn extract_url_from_input(expr: &ast::Expr) -> Option<String> {
    match expr {
        ast::Expr::Str(_) => string_value(expr),
        ast::Expr::AttrSet(attrset) => attrset.attrpath_values().find_map(|entry| {
            let attrpath = entry.attrpath()?;
            let name = attrpath_name(&attrpath)?;
            if name != "url" {
                return None;
            }
            let value = entry.value()?;
            string_value(&value)
        }),
        _ => None,
    }
}

fn check_no_manual_for_all_systems(source: &str, root: &Root) -> Vec<String> {
    root.syntax()
        .descendants()
        .filter_map(|node| {
            let select = ast::Select::cast(node)?;
            let text = select.syntax().text().to_string();
            if !text.contains("genAttrs") {
                return None;
            }
            let offset: usize = select.syntax().text_range().start().into();
            let line = line_number(source, offset);
            Some(format!(
                "line {line}: manual genAttrs instead of flake-utils"
            ))
        })
        .collect()
}

struct ParsedNixFile {
    path: String,
    root: Root,
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

fn parse(paths: Vec<String>) -> (Vec<ParsedNixFile>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => {
                let parse_result = Root::parse(&source);
                if parse_result.errors().is_empty() {
                    parsed.push(ParsedNixFile {
                        root: parse_result.tree(),
                        path,
                        source,
                    });
                } else {
                    errors.push(AppError::Parse {
                        file: path,
                        message: parse_result
                            .errors()
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                    });
                }
            }
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_rules(files: &[ParsedNixFile], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source, &file.root);
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

    fn parse_fixture(name: &str) -> (String, Root) {
        let path = format!("fixtures/nix/{name}.nix");
        let source = fs::read_to_string(&path).expect("fixture should exist");
        let root = Root::parse(&source).tree();
        (source, root)
    }

    #[test]
    fn no_unstable_nixpkgs_detects_violation() {
        let (source, root) = parse_fixture("no-unstable-nixpkgs");
        let errors = check_no_unstable_nixpkgs(&source, &root);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unstable_nixpkgs_allows_valid() {
        let (source, root) = parse_fixture("valid-no-unstable-nixpkgs");
        let errors = check_no_unstable_nixpkgs(&source, &root);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_manual_for_all_systems_detects_violation() {
        let (source, root) = parse_fixture("no-manual-for-all-systems");
        let errors = check_no_manual_for_all_systems(&source, &root);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_manual_for_all_systems_allows_valid() {
        let (source, root) = parse_fixture("valid-no-manual-for-all-systems");
        let errors = check_no_manual_for_all_systems(&source, &root);
        assert!(errors.is_empty());
    }
}
