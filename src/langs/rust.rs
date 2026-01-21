use std::fs;

use ra_ap_syntax::ast::HasName as _;
use ra_ap_syntax::{AstNode as _, SourceFile, SyntaxKind, ast};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["target", ".git", "fixtures"];
const PATTERNS: &[&str] = &["**/*.rs"];

struct Rule {
    check: fn(source: &str, ast: &SourceFile) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_impl,
        hint: Some("use free functions, impl is banned"),
        name: "no-impl",
    },
    Rule {
        check: check_no_trait_impl,
        hint: Some("trait impls are banned unless required by external crates"),
        name: "no-trait-impl",
    },
    Rule {
        check: check_no_print_outside_main,
        hint: Some("print statements should only be in main()"),
        name: "no-print-outside-main",
    },
];

struct ParsedFile {
    ast: SourceFile,
    path: String,
    source: String,
}

fn check_no_impl(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::IMPL {
                return None;
            }
            let impl_node = ast::Impl::cast(node)?;
            if impl_node.trait_().is_some() {
                return None;
            }
            let line = line_number(source, impl_node.syntax().text_range().start().into());
            let self_ty = impl_node
                .self_ty()
                .map_or_else(|| "block".to_string(), |ty| ty.to_string());
            Some(format!("line {line}: impl {self_ty}"))
        })
        .collect()
}

fn check_no_trait_impl(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::IMPL {
                return None;
            }
            let impl_node = ast::Impl::cast(node)?;
            let trait_ref = impl_node.trait_()?;
            let line = line_number(source, impl_node.syntax().text_range().start().into());
            let self_ty = impl_node
                .self_ty()
                .map_or_else(|| "block".to_string(), |ty| ty.to_string());
            Some(format!("line {line}: impl {trait_ref} for {self_ty}"))
        })
        .collect()
}

const PRINT_MACROS: &[&str] = &["println", "print", "eprintln", "eprint"];

fn is_inside_main(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        if ancestor.kind() != SyntaxKind::FN {
            return false;
        }
        let Some(func) = ast::Fn::cast(ancestor) else {
            return false;
        };
        func.name().is_some_and(|name| name.text() == "main")
    })
}

fn check_no_print_outside_main(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MACRO_CALL {
                return None;
            }
            let macro_call = ast::MacroCall::cast(node.clone())?;
            let macro_name = macro_call.path()?.segment()?.name_ref()?.text().to_string();
            if !PRINT_MACROS.contains(&macro_name.as_str()) {
                return None;
            }
            if is_inside_main(&node) {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: {macro_name}!"))
        })
        .collect()
}

fn line_number(source: &str, offset: usize) -> usize {
    source.get(..offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn find(patterns: &[&str], excludes: &[&str]) -> Vec<String> {
    patterns
        .iter()
        .flat_map(|pattern| find_files(pattern, excludes).unwrap_or_default())
        .collect()
}

fn parse(paths: Vec<String>) -> (Vec<ParsedFile>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => {
                let parse_result = SourceFile::parse(&source, ra_ap_syntax::Edition::Edition2024);
                parsed.push(ParsedFile {
                    ast: parse_result.tree(),
                    path,
                    source,
                });
            }
            Err(err) => {
                errors.push(AppError::ReadFailed {
                    file: path,
                    message: err.to_string(),
                });
            }
        }
    }

    (parsed, errors)
}

fn run_rules(files: &[ParsedFile], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source, &file.ast);
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
    let paths = find(PATTERNS, EXCLUDES);
    let (parsed, errors) = parse(paths);
    let violations = run_rules(&parsed, RULES);

    CheckResult {
        errors,
        file_count: parsed.len(),
        violations,
    }
}
