use std::fs;

use ra_ap_syntax::ast::{HasArgList as _, HasName as _};
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
        check: check_no_allow_attr,
        hint: Some("refactor code to satisfy the linter instead of suppressing"),
        name: "no-allow-attr",
    },
    Rule {
        check: check_no_catch_unwind,
        hint: Some("do not swallow panics with catch_unwind"),
        name: "no-catch-unwind",
    },
    Rule {
        check: check_no_cfg_attr_suppress,
        hint: Some("refactor code to satisfy the linter instead of suppressing"),
        name: "no-cfg-attr-suppress",
    },
    Rule {
        check: check_no_discard_result,
        hint: Some("handle the Result instead of discarding it with let _ ="),
        name: "no-discard-result",
    },
    Rule {
        check: check_no_drop_result,
        hint: Some("handle the Result instead of dropping it"),
        name: "no-drop-result",
    },
    Rule {
        check: check_no_empty_err_match,
        hint: Some("handle the error case instead of using an empty Err arm"),
        name: "no-empty-err-match",
    },
    Rule {
        check: check_no_expect_attr,
        hint: Some("refactor code to satisfy the linter instead of suppressing"),
        name: "no-expect-attr",
    },
    Rule {
        check: check_no_expect_outside_tests,
        hint: Some("use ? or match for error handling, expect is only allowed in tests"),
        name: "no-expect-outside-tests",
    },
    Rule {
        check: check_no_if_let_ok_without_else,
        hint: Some("add an else branch to handle the error case"),
        name: "no-if-let-ok-without-else",
    },
    Rule {
        check: check_no_impl,
        hint: Some("use free functions, impl is banned"),
        name: "no-impl",
    },
    Rule {
        check: check_no_is_ok_is_err,
        hint: Some("use the value or error instead of just checking with is_ok/is_err"),
        name: "no-is-ok-is-err",
    },
    Rule {
        check: check_no_map_err_discard,
        hint: Some("preserve the original error information in map_err"),
        name: "no-map-err-discard",
    },
    Rule {
        check: check_no_mem_forget,
        hint: Some("do not use mem::forget to leak resources"),
        name: "no-mem-forget",
    },
    Rule {
        check: check_no_ok_and_propagate,
        hint: Some("use ? on Result directly instead of .ok()? which loses error info"),
        name: "no-ok-and-propagate",
    },
    Rule {
        check: check_no_ok_method,
        hint: Some("handle the error explicitly instead of discarding it with .ok()"),
        name: "no-ok-method",
    },
    Rule {
        check: check_no_print_outside_main,
        hint: Some("print statements should only be in main()"),
        name: "no-print-outside-main",
    },
    Rule {
        check: check_no_trait_impl,
        hint: Some("trait impls are banned unless required by external crates"),
        name: "no-trait-impl",
    },
    Rule {
        check: check_no_unit_error,
        hint: Some("use a meaningful error type instead of ()"),
        name: "no-unit-error",
    },
    Rule {
        check: check_no_unwrap_err,
        hint: Some("handle the Ok case explicitly instead of unwrapping Err"),
        name: "no-unwrap-err",
    },
    Rule {
        check: check_no_unwrap_or,
        hint: Some("ask the user for approval if a default is necessary"),
        name: "no-unwrap-or",
    },
    Rule {
        check: check_no_unwrap_or_default,
        hint: Some("ask the user for approval if a default is necessary"),
        name: "no-unwrap-or-default",
    },
    Rule {
        check: check_no_unwrap_or_else,
        hint: Some("ask the user for approval if a default is necessary"),
        name: "no-unwrap-or-else",
    },
    Rule {
        check: check_no_unwrap_unchecked,
        hint: Some("use safe unwrap methods instead of unsafe unwrap_unchecked"),
        name: "no-unwrap-unchecked",
    },
    Rule {
        check: check_no_while_let_ok,
        hint: Some("while let Ok silently stops on errors, handle errors explicitly"),
        name: "no-while-let-ok",
    },
    Rule {
        check: check_no_wildcard_err,
        hint: Some("capture the error value instead of discarding it with Err(_)"),
        name: "no-wildcard-err",
    },
];

struct ParsedFile {
    ast: SourceFile,
    path: String,
    source: String,
}

const ALLOW_ATTR: &str = "allow";

fn check_no_allow_attr(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::ATTR {
                return None;
            }
            let attr = ast::Attr::cast(node)?;
            let path = attr.path()?;
            if path.to_string() != ALLOW_ATTR {
                return None;
            }
            let line = line_number(source, attr.syntax().text_range().start().into());
            Some(format!("line {line}: #[{ALLOW_ATTR}(...)]"))
        })
        .collect()
}

fn check_no_catch_unwind(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let call = ast::CallExpr::cast(node.clone())?;
            let expr = call.expr()?;
            let text = expr.to_string();
            let is_catch_unwind = text == "catch_unwind"
                || text == "panic::catch_unwind"
                || text == "std::panic::catch_unwind";
            if !is_catch_unwind {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: catch_unwind()"))
        })
        .collect()
}

fn check_no_cfg_attr_suppress(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::ATTR {
                return None;
            }
            let attr = ast::Attr::cast(node)?;
            let path = attr.path()?;
            if path.to_string() != "cfg_attr" {
                return None;
            }
            let token_tree = attr.token_tree()?;
            let text = token_tree.to_string();
            let has_allow = text.contains(ALLOW_ATTR);
            let has_expect = text.contains("expect");
            if !has_allow && !has_expect {
                return None;
            }
            let line = line_number(source, attr.syntax().text_range().start().into());
            let suppressed = if has_allow { ALLOW_ATTR } else { "expect" };
            Some(format!("line {line}: #[cfg_attr(..., {suppressed}(...))]"))
        })
        .collect()
}

fn check_no_discard_result(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::LET_STMT {
                return None;
            }
            let let_stmt = ast::LetStmt::cast(node.clone())?;
            let pat = let_stmt.pat()?;
            if !matches!(pat, ast::Pat::WildcardPat(_)) {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: let _ = ..."))
        })
        .collect()
}

fn check_no_drop_result(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let call = ast::CallExpr::cast(node.clone())?;
            let expr = call.expr()?;
            if expr.to_string() != "drop" {
                return None;
            }
            let arg_list = call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            if let ast::Expr::CallExpr(_) = first_arg {
                let line = line_number(source, node.text_range().start().into());
                return Some(format!("line {line}: drop(...)"));
            }
            None
        })
        .collect()
}

fn check_no_empty_err_match(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MATCH_ARM {
                return None;
            }
            let arm = ast::MatchArm::cast(node.clone())?;
            let pat = arm.pat()?;
            let pat_str = pat.to_string();
            if !pat_str.starts_with("Err") {
                return None;
            }
            let expr = arm.expr()?;
            let expr_str = expr.to_string();
            if expr_str != "{}" && expr_str != "()" {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: Err(_) => {{}}"))
        })
        .collect()
}

fn check_no_expect_attr(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::ATTR {
                return None;
            }
            let attr = ast::Attr::cast(node)?;
            let path = attr.path()?;
            if path.to_string() != "expect" {
                return None;
            }
            let line = line_number(source, attr.syntax().text_range().start().into());
            Some(format!("line {line}: #[expect(...)]"))
        })
        .collect()
}

fn is_inside_test_module(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        if ancestor.kind() != SyntaxKind::MODULE {
            return false;
        }
        let Some(module) = ast::Module::cast(ancestor.clone()) else {
            return false;
        };
        let is_test_module = module.name().is_some_and(|name| name.text() == "tests");
        let has_cfg_test = ancestor.children().any(|child| {
            if child.kind() != SyntaxKind::ATTR {
                return false;
            }
            child.to_string().contains("cfg(test)")
        });
        is_test_module && has_cfg_test
    })
}

fn check_no_expect_outside_tests(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let method_call = ast::MethodCallExpr::cast(node.clone())?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "expect" {
                return None;
            }
            if is_inside_test_module(&node) {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .expect()"))
        })
        .collect()
}

fn check_no_if_let_ok_without_else(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::IF_EXPR {
                return None;
            }
            let if_expr = ast::IfExpr::cast(node.clone())?;
            let condition = if_expr.condition()?;
            let ast::Expr::LetExpr(let_expr) = condition else {
                return None;
            };
            let pat = let_expr.pat()?;
            let pat_str = pat.to_string();
            if !pat_str.starts_with("Ok(") {
                return None;
            }
            if if_expr.else_branch().is_some() {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: if let Ok(...) without else"))
        })
        .collect()
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

fn check_no_is_ok_is_err(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let method_call = ast::MethodCallExpr::cast(node.clone())?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "is_ok" && method_name != "is_err" {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .{method_name}()"))
        })
        .collect()
}

fn check_no_map_err_discard(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let method_call = ast::MethodCallExpr::cast(node.clone())?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "map_err" {
                return None;
            }
            let arg_list = method_call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            let arg_str = first_arg.to_string();
            if !arg_str.contains("|_|") {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .map_err(|_| ...)"))
        })
        .collect()
}

fn check_no_mem_forget(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let call = ast::CallExpr::cast(node.clone())?;
            let expr = call.expr()?;
            let text = expr.to_string();
            if text != "mem::forget" && text != "std::mem::forget" && text != "forget" {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: mem::forget()"))
        })
        .collect()
}

fn check_no_ok_and_propagate(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::TRY_EXPR {
                return None;
            }
            let try_expr = ast::TryExpr::cast(node.clone())?;
            let inner = try_expr.expr()?;
            let ast::Expr::MethodCallExpr(method_call) = inner else {
                return None;
            };
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "ok" {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .ok()?"))
        })
        .collect()
}

fn check_no_ok_method(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let method_call = ast::MethodCallExpr::cast(node.clone())?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "ok" {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .ok()"))
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

fn check_no_unwrap_err(source: &str, ast: &SourceFile) -> Vec<String> {
    check_method_call(source, ast, "unwrap_err")
}

fn check_no_unwrap_or(source: &str, ast: &SourceFile) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or")
}

fn check_no_unwrap_or_default(source: &str, ast: &SourceFile) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or_default")
}

fn check_no_unwrap_or_else(source: &str, ast: &SourceFile) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or_else")
}

fn check_no_unwrap_unchecked(source: &str, ast: &SourceFile) -> Vec<String> {
    check_method_call(source, ast, "unwrap_unchecked")
}

fn check_method_call(source: &str, ast: &SourceFile, method: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let method_call = ast::MethodCallExpr::cast(node.clone())?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != method {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: .{method_name}()"))
        })
        .collect()
}

fn check_no_unit_error(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::PATH_TYPE {
                return None;
            }
            let type_str = node.to_string();
            if !type_str.starts_with("Result<") {
                return None;
            }
            if !type_str.contains(", ()>") && !type_str.ends_with(", ()>") {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: Result<_, ()>"))
        })
        .collect()
}

fn check_no_while_let_ok(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::WHILE_EXPR {
                return None;
            }
            let while_expr = ast::WhileExpr::cast(node.clone())?;
            let condition = while_expr.condition()?;
            let ast::Expr::LetExpr(let_expr) = condition else {
                return None;
            };
            let pat = let_expr.pat()?;
            let pat_str = pat.to_string();
            if !pat_str.starts_with("Ok(") {
                return None;
            }
            let line = line_number(source, node.text_range().start().into());
            Some(format!("line {line}: while let Ok(...)"))
        })
        .collect()
}

fn check_no_wildcard_err(source: &str, ast: &SourceFile) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::TUPLE_STRUCT_PAT {
                return None;
            }
            let pat = ast::TupleStructPat::cast(node)?;
            let path = pat.path()?;
            if path.to_string() != "Err" {
                return None;
            }
            let has_wildcard = pat
                .fields()
                .any(|field| matches!(field, ast::Pat::WildcardPat(_)));
            if !has_wildcard {
                return None;
            }
            let line = line_number(source, pat.syntax().text_range().start().into());
            Some(format!("line {line}: Err(_)"))
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

fn find(patterns: &[&str], excludes: &[&str]) -> (Vec<String>, Vec<AppError>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for pattern in patterns {
        match find_files(pattern, excludes) {
            Ok(found) => files.extend(found),
            Err(err) => errors.push(AppError::Glob {
                pattern: (*pattern).to_string(),
                message: err.to_string(),
            }),
        }
    }
    (files, errors)
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
                errors.push(AppError::Read {
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
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let (parsed, parse_errors) = parse(paths);
    errors.extend(parse_errors);
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

    fn parse_fixture(name: &str) -> (String, SourceFile) {
        let path = format!("fixtures/rust/{name}.rs");
        let source = fs::read_to_string(&path).expect("fixture should exist");
        let parse_result = SourceFile::parse(&source, ra_ap_syntax::Edition::Edition2024);
        (source, parse_result.tree())
    }

    #[test]
    fn no_allow_attr_detects_violation() {
        let (source, ast) = parse_fixture("no-allow-attr");
        let errors = check_no_allow_attr(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_allow_attr_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_allow_attr(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_catch_unwind_detects_violation() {
        let (source, ast) = parse_fixture("no-catch-unwind");
        let errors = check_no_catch_unwind(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_catch_unwind_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-catch-unwind");
        let errors = check_no_catch_unwind(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_cfg_attr_suppress_detects_violation() {
        let (source, ast) = parse_fixture("no-cfg-attr-suppress");
        let errors = check_no_cfg_attr_suppress(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_cfg_attr_suppress_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_cfg_attr_suppress(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_discard_result_detects_violation() {
        let (source, ast) = parse_fixture("no-discard-result");
        let errors = check_no_discard_result(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_discard_result_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-discard-result");
        let errors = check_no_discard_result(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_drop_result_detects_violation() {
        let (source, ast) = parse_fixture("no-drop-result");
        let errors = check_no_drop_result(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_drop_result_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-drop-result");
        let errors = check_no_drop_result(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_empty_err_match_detects_violation() {
        let (source, ast) = parse_fixture("no-empty-err-match");
        let errors = check_no_empty_err_match(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_empty_err_match_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-empty-err-match");
        let errors = check_no_empty_err_match(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_expect_attr_detects_violation() {
        let (source, ast) = parse_fixture("no-expect-attr");
        let errors = check_no_expect_attr(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_expect_attr_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_expect_attr(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_expect_outside_tests_detects_violation() {
        let (source, ast) = parse_fixture("no-expect-outside-tests");
        let errors = check_no_expect_outside_tests(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_expect_outside_tests_allows_valid() {
        let (source, ast) = parse_fixture("valid-expect-in-tests");
        let errors = check_no_expect_outside_tests(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_if_let_ok_without_else_detects_violation() {
        let (source, ast) = parse_fixture("no-if-let-ok-without-else");
        let errors = check_no_if_let_ok_without_else(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_if_let_ok_without_else_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-if-let-ok-without-else");
        let errors = check_no_if_let_ok_without_else(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_impl_detects_violation() {
        let (source, ast) = parse_fixture("no-impl");
        let errors = check_no_impl(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_is_ok_is_err_detects_violation() {
        let (source, ast) = parse_fixture("no-is-ok-is-err");
        let errors = check_no_is_ok_is_err(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_is_ok_is_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-is-ok-is-err");
        let errors = check_no_is_ok_is_err(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_map_err_discard_detects_violation() {
        let (source, ast) = parse_fixture("no-map-err-discard");
        let errors = check_no_map_err_discard(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_map_err_discard_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-map-err-discard");
        let errors = check_no_map_err_discard(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_mem_forget_detects_violation() {
        let (source, ast) = parse_fixture("no-mem-forget");
        let errors = check_no_mem_forget(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_mem_forget_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-mem-forget");
        let errors = check_no_mem_forget(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ok_and_propagate_detects_violation() {
        let (source, ast) = parse_fixture("no-ok-and-propagate");
        let errors = check_no_ok_and_propagate(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ok_and_propagate_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-ok-and-propagate");
        let errors = check_no_ok_and_propagate(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ok_method_detects_violation() {
        let (source, ast) = parse_fixture("no-ok-method");
        let errors = check_no_ok_method(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ok_method_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-ok-method");
        let errors = check_no_ok_method(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_trait_impl_detects_violation() {
        let (source, ast) = parse_fixture("no-trait-impl");
        let errors = check_no_trait_impl(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_print_outside_main_detects_violation() {
        let (source, ast) = parse_fixture("no-print-outside-main");
        let errors = check_no_print_outside_main(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_print_outside_main_allows_valid() {
        let (source, ast) = parse_fixture("valid-print-in-main");
        let errors = check_no_print_outside_main(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_err_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-err");
        let errors = check_no_unwrap_err(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_err(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or");
        let errors = check_no_unwrap_or(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_default_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or-default");
        let errors = check_no_unwrap_or_default(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_default_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or_default(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_else_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or-else");
        let errors = check_no_unwrap_or_else(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_else_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or_else(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_unchecked_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-unchecked");
        let errors = check_no_unwrap_unchecked(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_unchecked_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_unchecked(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unit_error_detects_violation() {
        let (source, ast) = parse_fixture("no-unit-error");
        let errors = check_no_unit_error(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unit_error_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unit-error");
        let errors = check_no_unit_error(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_while_let_ok_detects_violation() {
        let (source, ast) = parse_fixture("no-while-let-ok");
        let errors = check_no_while_let_ok(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_while_let_ok_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-while-let-ok");
        let errors = check_no_while_let_ok(&source, &ast);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_wildcard_err_detects_violation() {
        let (source, ast) = parse_fixture("no-wildcard-err");
        let errors = check_no_wildcard_err(&source, &ast);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_wildcard_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-wildcard-err");
        let errors = check_no_wildcard_err(&source, &ast);
        assert!(errors.is_empty());
    }
}
