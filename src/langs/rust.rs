use std::{collections::BTreeMap, fs, path::Path};

use ra_ap_syntax::ast::{HasArgList as _, HasAttrs as _, HasName as _, HasVisibility as _};
use ra_ap_syntax::{AstNode as _, SourceFile, SyntaxKind, ast};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["fixtures"];
const PATTERNS: &[&str] = &["*.rs"];
const CARGO_TOML_PATTERNS: &[&str] = &["Cargo.toml"];

struct Rule {
    check: fn(source: &str, ast: &SourceFile, path: &str) -> Vec<String>,
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
        check: check_no_discard_try_value,
        hint: Some("check the success value instead of discarding it"),
        name: "no-discard-try-value",
    },
    Rule {
        check: check_no_drop_result,
        hint: Some("handle the Result instead of dropping it"),
        name: "no-drop-result",
    },
    Rule {
        check: check_no_direct_env_access,
        hint: Some("no direct env access, env should only be accessed through config.rs or env.rs"),
        name: "no-direct-env-access",
    },
    Rule {
        check: check_no_nested_call_args,
        hint: Some("assign the inner call to a local before passing it"),
        name: "no-nested-call-args",
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
        check: check_no_let_ok_else_continue,
        hint: Some("silently skips errors - return Result from function and use ? instead"),
        name: "no-let-ok-else-continue",
    },
    Rule {
        check: check_no_ignore_test,
        hint: Some("don't ignore tests, fix the underlying issue"),
        name: "no-ignore-test",
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
        check: check_no_map_discard,
        hint: Some("do not hide errors by discarding the value"),
        name: "no-map-discard",
    },
    Rule {
        check: check_no_map_err_discard,
        hint: Some("preserve the original error information in map_err"),
        name: "no-map-err-discard",
    },
    Rule {
        check: check_no_map_or_else,
        hint: Some(
            "use match for explicit handling or convert Option to Result with .ok_or_else()",
        ),
        name: "no-map-or-else",
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
        check: check_no_pub_crate_fn,
        hint: Some("use pub instead, the project is a single crate"),
        name: "no-pub-crate-fn",
    },
    Rule {
        check: check_no_return_ok,
        hint: Some("return Ok(()) hides errors, handle the condition properly"),
        name: "no-return-ok",
    },
    Rule {
        check: check_no_spawn,
        hint: Some("use .output() or .status() to capture and handle command results"),
        name: "no-spawn",
    },
    Rule {
        check: check_no_sqlx_query_outside_models,
        hint: Some("keep sqlx query construction inside models/"),
        name: "no-sqlx-query-outside-models",
    },
    Rule {
        check: check_no_sqlx_query_without_tracing_instrument,
        hint: Some("add #[tracing::instrument] to the enclosing function"),
        name: "no-sqlx-query-without-tracing-instrument",
    },
    Rule {
        check: check_no_sqlx_query_without_skip_pool,
        hint: Some("add skip(pool) to #[tracing::instrument(...)] when pool is a parameter"),
        name: "no-sqlx-query-without-skip-pool",
    },
    Rule {
        check: check_no_stderr_null,
        hint: Some("do not silence stderr, capture and handle it"),
        name: "no-stderr-null",
    },
    Rule {
        check: check_no_trait_impl,
        hint: Some("trait impls are banned unless required by external crates"),
        name: "no-trait-impl",
    },
    Rule {
        check: check_thiserror_unused_fields,
        hint: Some("all named fields should appear in the #[error(\"...\")] format string"),
        name: "thiserror-unused-fields",
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
        check: check_no_inline_style,
        hint: Some("use CSS classes instead of inline styles"),
        name: "no-inline-style",
    },
    Rule {
        check: check_no_swallowed_err,
        hint: Some("Err matched but not returned - error is being swallowed"),
        name: "no-swallowed-err",
    },
];

struct CargoTomlRule {
    check: fn(toml: &toml::Value) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const CARGO_TOML_RULES: &[CargoTomlRule] = &[CargoTomlRule {
    check: check_pinned_versions,
    hint: Some("specify full semver version (e.g., \"1.0.95\" instead of \"1\" or \"1.0\")"),
    name: "pinned-versions",
}];

struct ParsedCargoToml {
    path: String,
    toml: toml::Value,
}

struct ParsedFile {
    ast: SourceFile,
    path: String,
    source: String,
}

fn is_incomplete_version(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    parts.len() < 3 || parts.iter().any(|part| part.is_empty())
}

fn check_pinned_versions(toml: &toml::Value) -> Vec<String> {
    let mut errors = Vec::new();

    let dep_sections = ["dependencies", "dev-dependencies", "build-dependencies"];

    for section in dep_sections {
        let Some(deps) = toml.get(section).and_then(toml::Value::as_table) else {
            continue;
        };

        for (name, value) in deps {
            let version = match value {
                toml::Value::String(ver) => {
                    let version_str = ver.as_str();
                    Some(version_str)
                }
                toml::Value::Table(table) => table.get("version").and_then(toml::Value::as_str),
                _ => None,
            };

            if let Some(ver) = version
                && is_incomplete_version(ver)
            {
                errors.push(format!("{section}.{name}: \"{ver}\" incomplete version"));
            }
        }
    }

    errors
}

const ALLOW_ATTR: &str = "allow";
const ALLOWED_ENV_ACCESS_FILES: &[&str] = &["build.rs", "config.rs", "env.rs"];

fn check_no_allow_attr(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
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
            let syntax = attr.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: #[{ALLOW_ATTR}(...)]"))
        })
        .collect()
}

fn check_no_catch_unwind(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let call = ast::CallExpr::cast(syntax_node)?;
            let expr = call.expr()?;
            let text = expr.to_string();
            let is_catch_unwind = text == "catch_unwind"
                || text == "panic::catch_unwind"
                || text == "std::panic::catch_unwind";
            if !is_catch_unwind {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: catch_unwind()"))
        })
        .collect()
}

fn check_no_cfg_attr_suppress(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
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
            let syntax = attr.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            let suppressed = if has_allow { ALLOW_ATTR } else { "expect" };
            Some(format!("line {line}: #[cfg_attr(..., {suppressed}(...))]"))
        })
        .collect()
}

fn is_call_expr(expr: &ast::Expr) -> bool {
    matches!(
        expr,
        ast::Expr::CallExpr(_) | ast::Expr::MethodCallExpr(_) | ast::Expr::AwaitExpr(_)
    )
}

fn check_no_discard_result(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::LET_STMT {
                return None;
            }
            let syntax_node = node.clone();
            let let_stmt = ast::LetStmt::cast(syntax_node)?;
            let pat = let_stmt.pat()?;
            let is_wildcard = matches!(pat, ast::Pat::WildcardPat(_));
            let is_underscore_ident = matches!(&pat, ast::Pat::IdentPat(ident) if {
                ident.name().is_some_and(|n| {
                    let text = n.text().to_string();
                    text.starts_with('_') && text != "_"
                })
            });
            if !is_wildcard && !is_underscore_ident {
                return None;
            }
            if is_underscore_ident {
                let init = let_stmt.initializer()?;
                if !is_call_expr(&init) {
                    return None;
                }
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            let binding = match &pat {
                ast::Pat::IdentPat(ident) => match ident.name() {
                    Some(n) => n.text().to_string(),
                    None => "_".to_string(),
                },
                _ => "_".to_string(),
            };
            Some(format!("line {line}: let {binding} = ..."))
        })
        .collect()
}

fn check_no_discard_try_value(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::LET_STMT {
                return None;
            }
            let syntax_node = node.clone();
            let let_stmt = ast::LetStmt::cast(syntax_node)?;
            let pat = let_stmt.pat()?;
            let is_wildcard = matches!(pat, ast::Pat::WildcardPat(_));
            let is_underscore_ident = matches!(&pat, ast::Pat::IdentPat(ident) if {
                ident.name().is_some_and(|n| n.text().starts_with('_'))
            });
            if !is_wildcard && !is_underscore_ident {
                return None;
            }
            let init = let_stmt.initializer()?;
            if !matches!(init, ast::Expr::TryExpr(_)) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            let binding = match &pat {
                ast::Pat::IdentPat(ident) => match ident.name() {
                    Some(n) => n.text().to_string(),
                    None => "_".to_string(),
                },
                _ => "_".to_string(),
            };
            Some(format!("line {line}: let {binding} = ...?"))
        })
        .collect()
}

fn check_no_drop_result(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let call = ast::CallExpr::cast(syntax_node)?;
            let expr = call.expr()?;
            if expr.to_string() != "drop" {
                return None;
            }
            let arg_list = call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            if let ast::Expr::CallExpr(_) = first_arg {
                let start = node.text_range().start();
                let offset: usize = start.into();
                let line = line_number(source, offset);
                return Some(format!("line {line}: drop(...)"));
            }
            None
        })
        .collect()
}

fn is_allowed_env_access_file(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| ALLOWED_ENV_ACCESS_FILES.contains(&name))
}

fn is_in_models_dir(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|component| component.as_os_str().to_str() == Some("models"))
}

fn normalize_path_text(path: &str) -> String {
    path.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn extract_sqlx_name(path: &str) -> Option<String> {
    let normalized = normalize_path_text(path);
    let trimmed = normalized.trim_start_matches("::");
    let mut segments = trimmed.split("::");
    if segments.next()? != "sqlx" {
        return None;
    }
    let name = segments.next()?.split('<').next()?.to_string();
    Some(name)
}

fn is_sqlx_query_name(name: &str) -> bool {
    name == "query" || (name.starts_with("query_") && name != "query_builder")
}

fn is_sqlx_query_call(node: &ra_ap_syntax::SyntaxNode) -> bool {
    let syntax_node = node.clone();
    let Some(call) = ast::CallExpr::cast(syntax_node) else {
        return false;
    };
    let Some(ast::Expr::PathExpr(path_expr)) = call.expr() else {
        return false;
    };
    let Some(path) = path_expr.path() else {
        return false;
    };
    let path_text = path.to_string();
    let Some(name) = extract_sqlx_name(&path_text) else {
        return false;
    };
    is_sqlx_query_name(&name)
}

fn is_sqlx_query_macro(node: &ra_ap_syntax::SyntaxNode) -> bool {
    let syntax_node = node.clone();
    let Some(macro_call) = ast::MacroCall::cast(syntax_node) else {
        return false;
    };
    let Some(path) = macro_call.path() else {
        return false;
    };
    let path_text = path.to_string();
    let Some(name) = extract_sqlx_name(&path_text) else {
        return false;
    };
    is_sqlx_query_name(&name)
}

fn is_sqlx_query_node(node: &ra_ap_syntax::SyntaxNode) -> bool {
    match node.kind() {
        SyntaxKind::CALL_EXPR => is_sqlx_query_call(node),
        SyntaxKind::MACRO_CALL => is_sqlx_query_macro(node),
        _ => false,
    }
}

fn enclosing_fn(node: &ra_ap_syntax::SyntaxNode) -> Option<ast::Fn> {
    node.ancestors().find_map(ast::Fn::cast)
}

fn has_pool_param(func: &ast::Fn) -> bool {
    let Some(param_list) = func.param_list() else {
        return false;
    };
    param_list.params().any(|param| {
        let Some(ast::Pat::IdentPat(ident)) = param.pat() else {
            return false;
        };
        ident.name().is_some_and(|name| name.text() == "pool")
    })
}

fn is_instrument_attr(attr: &ast::Attr) -> bool {
    let Some(path) = attr.path() else {
        return false;
    };
    let path_text = path.to_string();
    let normalized = normalize_path_text(&path_text);
    let trimmed = normalized.trim_start_matches("::");
    trimmed == "instrument" || trimmed == "tracing::instrument"
}

fn attr_skips_pool(attr: &ast::Attr) -> bool {
    let Some(token_tree) = attr.token_tree() else {
        return false;
    };
    let token_tree_text = token_tree.to_string();
    let normalized = normalize_path_text(&token_tree_text);
    normalized.contains("skip(pool")
}

fn fn_name(func: &ast::Fn) -> String {
    match func.name() {
        Some(name) => name.text().to_string(),
        None => "?".to_string(),
    }
}

fn has_instrument_attr(func: &ast::Fn) -> bool {
    func.attrs().any(|attr| is_instrument_attr(&attr))
}

fn has_instrument_attr_with_pool_skip(func: &ast::Fn) -> bool {
    func.attrs()
        .any(|attr| is_instrument_attr(&attr) && attr_skips_pool(&attr))
}

fn missing_tracing_instrument_error(
    source: &str,
    node: &ra_ap_syntax::SyntaxNode,
) -> Option<(usize, String)> {
    let func = enclosing_fn(node)?;
    if has_instrument_attr(&func) {
        return None;
    }
    let start = func.syntax().text_range().start();
    let offset: usize = start.into();
    let line = line_number(source, offset);
    let fn_name = fn_name(&func);
    Some((
        offset,
        format!("line {line}: fn {fn_name} missing #[tracing::instrument]"),
    ))
}

fn missing_pool_skip_error(
    source: &str,
    node: &ra_ap_syntax::SyntaxNode,
) -> Option<(usize, String)> {
    let func = enclosing_fn(node)?;
    if !has_pool_param(&func)
        || !has_instrument_attr(&func)
        || has_instrument_attr_with_pool_skip(&func)
    {
        return None;
    }
    let start = func.syntax().text_range().start();
    let offset: usize = start.into();
    let line = line_number(source, offset);
    let fn_name = fn_name(&func);
    Some((
        offset,
        format!("line {line}: fn {fn_name} missing skip(pool) in #[tracing::instrument(...)]"),
    ))
}

fn collect_sqlx_query_fn_errors(
    source: &str,
    ast: &SourceFile,
    error_for_query: fn(&str, &ra_ap_syntax::SyntaxNode) -> Option<(usize, String)>,
) -> Vec<String> {
    let mut errors = BTreeMap::new();

    for node in ast.syntax().descendants() {
        if !is_sqlx_query_node(&node) {
            continue;
        }
        let Some((offset, error)) = error_for_query(source, &node) else {
            continue;
        };
        errors.entry(offset).or_insert(error);
    }

    errors.into_values().collect()
}

fn check_no_direct_env_access(source: &str, ast: &SourceFile, path: &str) -> Vec<String> {
    if is_allowed_env_access_file(path) {
        return vec![];
    }

    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let call = ast::CallExpr::cast(syntax_node)?;
            let ast::Expr::PathExpr(path_expr) = call.expr()? else {
                return None;
            };
            let call_path = path_expr.path()?.to_string();
            let normalized_path: String =
                call_path.chars().filter(|ch| !ch.is_whitespace()).collect();
            let is_direct_env_access = matches!(
                normalized_path.as_str(),
                "std::env::var" | "::std::env::var" | "env::var"
            );
            if !is_direct_env_access {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: {normalized_path}()"))
        })
        .collect()
}

fn is_nested_call_expr(expr: &ast::Expr) -> bool {
    match expr {
        ast::Expr::AwaitExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        ast::Expr::CallExpr(_) | ast::Expr::MethodCallExpr(_) => true,
        ast::Expr::CastExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        ast::Expr::ParenExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        ast::Expr::PrefixExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        ast::Expr::RefExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        ast::Expr::TryExpr(inner) => inner
            .expr()
            .is_some_and(|inner_expr| is_nested_call_expr(&inner_expr)),
        _ => false,
    }
}

fn check_no_nested_call_args(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| match node.kind() {
            SyntaxKind::CALL_EXPR => {
                let syntax_node = node.clone();
                let call = ast::CallExpr::cast(syntax_node)?;
                let has_nested = call.arg_list()?.args().any(|arg| is_nested_call_expr(&arg));
                if !has_nested {
                    return None;
                }
                let start = node.text_range().start();
                let offset: usize = start.into();
                let line = line_number(source, offset);
                Some(format!("line {line}: nested call argument"))
            }
            SyntaxKind::METHOD_CALL_EXPR => {
                let syntax_node = node.clone();
                let call = ast::MethodCallExpr::cast(syntax_node)?;
                let has_nested = call.arg_list()?.args().any(|arg| is_nested_call_expr(&arg));
                if !has_nested {
                    return None;
                }
                let start = node.text_range().start();
                let offset: usize = start.into();
                let line = line_number(source, offset);
                Some(format!("line {line}: nested call argument"))
            }
            _ => None,
        })
        .collect()
}

fn check_no_expect_attr(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
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
            let syntax = attr.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: #[expect(...)]"))
        })
        .collect()
}

fn is_inside_test_module(node: &ra_ap_syntax::SyntaxNode) -> bool {
    node.ancestors().any(|ancestor| {
        if ancestor.kind() != SyntaxKind::MODULE {
            return false;
        }
        let syntax_node = ancestor.clone();
        let Some(module) = ast::Module::cast(syntax_node) else {
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

fn is_integration_test(path: &str) -> bool {
    path.contains("/tests/") || path.starts_with("tests/") || path.starts_with("./tests/")
}

fn check_no_expect_outside_tests(source: &str, ast: &SourceFile, path: &str) -> Vec<String> {
    if is_integration_test(path) {
        return vec![];
    }
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "expect" {
                return None;
            }
            if is_inside_test_module(&node) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .expect()"))
        })
        .collect()
}

fn check_no_if_let_ok_without_else(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::IF_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let if_expr = ast::IfExpr::cast(syntax_node)?;
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
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: if let Ok(...) without else"))
        })
        .collect()
}

fn check_no_let_ok_else_continue(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::LET_STMT {
                return None;
            }
            let syntax_node = node.clone();
            let let_stmt = ast::LetStmt::cast(syntax_node)?;
            let pat = let_stmt.pat()?;
            let pat_str = pat.to_string();
            if !pat_str.starts_with("Ok(") {
                return None;
            }
            let let_else = let_stmt.let_else()?;
            let block = let_else.block_expr()?;
            let block_str = block.to_string();
            if !block_str.contains("continue") {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: let Ok(...) else {{ continue }}"))
        })
        .collect()
}

fn check_no_ignore_test(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::ATTR {
                return None;
            }
            let attr = ast::Attr::cast(node)?;
            let path = attr.path()?;
            if path.to_string() != "ignore" {
                return None;
            }
            let syntax = attr.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: #[ignore]"))
        })
        .collect()
}

fn check_no_impl(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
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
            let syntax = impl_node.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            let self_ty = match impl_node.self_ty() {
                Some(ty) => ty.to_string(),
                None => "block".to_string(),
            };
            Some(format!("line {line}: impl {self_ty}"))
        })
        .collect()
}

fn check_no_is_ok_is_err(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "is_ok" && method_name != "is_err" {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .{method_name}()"))
        })
        .collect()
}

fn check_no_map_discard(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "map" {
                return None;
            }
            let arg_list = method_call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            let arg_str = first_arg.to_string();
            if !arg_str.contains("|_|") {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .map(|_| ...)"))
        })
        .collect()
}

fn has_discarded_closure_param(arg_str: &str) -> bool {
    let mut chars = arg_str.chars().peekable();
    for ch in chars.by_ref() {
        if ch == '|' {
            break;
        }
    }
    while chars.peek() == Some(&' ') {
        chars.next();
    }
    chars.peek() == Some(&'_')
}

fn check_no_map_err_discard(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "map_err" {
                return None;
            }
            let arg_list = method_call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            let arg_str = first_arg.to_string();
            if !has_discarded_closure_param(&arg_str) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .map_err(|_| ...)"))
        })
        .collect()
}

fn check_no_map_or_else(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "map_or_else")
}

fn check_no_mem_forget(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let call = ast::CallExpr::cast(syntax_node)?;
            let expr = call.expr()?;
            let text = expr.to_string();
            if text != "mem::forget" && text != "std::mem::forget" && text != "forget" {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: mem::forget()"))
        })
        .collect()
}

fn check_no_ok_and_propagate(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::TRY_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let try_expr = ast::TryExpr::cast(syntax_node)?;
            let inner = try_expr.expr()?;
            let ast::Expr::MethodCallExpr(method_call) = inner else {
                return None;
            };
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "ok" {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .ok()?"))
        })
        .collect()
}

fn check_no_ok_method(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "ok" {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .ok()"))
        })
        .collect()
}

fn check_no_trait_impl(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::IMPL {
                return None;
            }
            let impl_node = ast::Impl::cast(node)?;
            let trait_ref = impl_node.trait_()?;
            let syntax = impl_node.syntax();
            let start = syntax.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            let self_ty = match impl_node.self_ty() {
                Some(ty) => ty.to_string(),
                None => "block".to_string(),
            };
            Some(format!("line {line}: impl {trait_ref} for {self_ty}"))
        })
        .collect()
}

fn has_thiserror_derive(enum_node: &ast::Enum) -> bool {
    enum_node.syntax().children().any(|child| {
        if child.kind() != SyntaxKind::ATTR {
            return false;
        }
        let Some(attr) = ast::Attr::cast(child) else {
            return false;
        };
        let Some(path) = attr.path() else {
            return false;
        };
        if path.to_string() != "derive" {
            return false;
        }
        let Some(token_tree) = attr.token_tree() else {
            return false;
        };
        let text = token_tree.to_string();
        text.contains("thiserror::Error") || text.contains("thiserror :: Error")
    })
}

fn extract_error_attr_format(variant: &ast::Variant) -> Option<String> {
    variant.syntax().children().find_map(|child| {
        if child.kind() != SyntaxKind::ATTR {
            return None;
        }
        let attr = ast::Attr::cast(child)?;
        let path = attr.path()?;
        if path.to_string() != "error" {
            return None;
        }
        let token_tree = attr.token_tree()?;
        let token_text = token_tree.to_string();
        Some(token_text)
    })
}

fn extract_format_placeholders(format_str: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
    let mut chars = format_str.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                continue;
            }
            let mut name = String::new();
            for inner_ch in chars.by_ref() {
                if inner_ch == '}' || inner_ch == ':' {
                    break;
                }
                name.push(inner_ch);
            }
            if !name.is_empty()
                && !name
                    .chars()
                    .next()
                    .is_some_and(|first| first.is_ascii_digit())
            {
                placeholders.push(name);
            }
        }
    }
    placeholders
}

fn check_thiserror_unused_fields(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    let mut errors = Vec::new();

    for node in ast.syntax().descendants() {
        if node.kind() != SyntaxKind::ENUM {
            continue;
        }
        let Some(enum_node) = ast::Enum::cast(node) else {
            continue;
        };
        if !has_thiserror_derive(&enum_node) {
            continue;
        }
        let Some(variant_list) = enum_node.variant_list() else {
            continue;
        };
        for variant in variant_list.variants() {
            let Some(field_list) = variant.field_list() else {
                continue;
            };
            let ast::FieldList::RecordFieldList(record_fields) = field_list else {
                continue;
            };
            let field_names: Vec<String> = record_fields
                .fields()
                .filter_map(|field| field.name().map(|name| name.text().to_string()))
                .collect();
            if field_names.is_empty() {
                continue;
            }
            let Some(format_str) = extract_error_attr_format(&variant) else {
                continue;
            };
            let placeholders = extract_format_placeholders(&format_str);
            for field_name in &field_names {
                if !placeholders.contains(field_name) {
                    let syntax = variant.syntax();
                    let start = syntax.text_range().start();
                    let offset: usize = start.into();
                    let line = line_number(source, offset);
                    let variant_name = match variant.name() {
                        Some(name) => name.text().to_string(),
                        None => "?".to_string(),
                    };
                    errors.push(format!(
                        "line {line}: {variant_name}::{field_name} not in #[error(\"...\")]"
                    ));
                }
            }
        }
    }

    errors
}

fn check_no_unwrap_err(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "unwrap_err")
}

fn check_no_unwrap_or(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or")
}

fn check_no_unwrap_or_default(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or_default")
}

fn check_no_unwrap_or_else(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "unwrap_or_else")
}

fn check_no_unwrap_unchecked(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "unwrap_unchecked")
}

fn check_method_call(source: &str, ast: &SourceFile, method: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != method {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .{method_name}()"))
        })
        .collect()
}

fn check_no_unit_error(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
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
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: Result<_, ()>"))
        })
        .collect()
}

fn check_no_while_let_ok(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::WHILE_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let while_expr = ast::WhileExpr::cast(syntax_node)?;
            let condition = while_expr.condition()?;
            let ast::Expr::LetExpr(let_expr) = condition else {
                return None;
            };
            let pat = let_expr.pat()?;
            let pat_str = pat.to_string();
            if !pat_str.starts_with("Ok(") {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: while let Ok(...)"))
        })
        .collect()
}

fn extract_err_binding(pat: &ast::Pat) -> Option<String> {
    let ast::Pat::TupleStructPat(tuple_pat) = pat else {
        return None;
    };
    if tuple_pat.path()?.to_string() != "Err" {
        return None;
    }
    let field = tuple_pat.fields().next()?;
    let field_name = field.to_string();
    Some(field_name)
}

fn has_expect(expr_str: &str) -> bool {
    expr_str.contains(".expect(")
}

fn check_no_inline_style(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MACRO_CALL {
                return None;
            }
            let tokens: Vec<_> = node
                .descendants_with_tokens()
                .filter_map(ra_ap_syntax::NodeOrToken::into_token)
                .collect();
            let trailing_tokens = tokens.iter().skip(1);
            let token_pairs = tokens.iter().zip(trailing_tokens);
            let found: Vec<_> = token_pairs
                .filter(|(ident, eq)| {
                    ident.kind() == SyntaxKind::IDENT
                        && ident.text() == "style"
                        && eq.kind() == SyntaxKind::EQ
                })
                .map(|(ident, _)| {
                    let start = ident.text_range().start();
                    let offset: usize = start.into();
                    let line = line_number(source, offset);
                    format!("line {line}: style=")
                })
                .collect();
            if found.is_empty() { None } else { Some(found) }
        })
        .flatten()
        .collect()
}

fn check_no_swallowed_err(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MATCH_ARM {
                return None;
            }
            let syntax_node = node.clone();
            let arm = ast::MatchArm::cast(syntax_node)?;
            let pat = arm.pat()?;
            let binding = extract_err_binding(&pat)?;
            let expr = arm.expr()?;
            let expr_str = expr.to_string();
            if has_expect(&expr_str) {
                return None;
            }
            let is_discarded = binding.starts_with('_');
            let is_unused = !expr_str.contains(&binding);
            if !is_discarded && !is_unused {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: Err({binding}) not used"))
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

fn check_no_print_outside_main(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MACRO_CALL {
                return None;
            }
            let syntax_node = node.clone();
            let macro_call = ast::MacroCall::cast(syntax_node)?;
            let macro_name = macro_call.path()?.segment()?.name_ref()?.text().to_string();
            let macro_name_ref = macro_name.as_str();
            if !PRINT_MACROS.contains(&macro_name_ref) {
                return None;
            }
            if is_inside_main(&node) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: {macro_name}!"))
        })
        .collect()
}

fn check_no_pub_crate_fn(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::FN {
                return None;
            }
            let func = ast::Fn::cast(node)?;
            let vis = func.visibility()?;
            if vis.syntax().to_string() != "pub(crate)" {
                return None;
            }
            let name = func.name()?;
            let offset: usize = vis.syntax().text_range().start().into();
            let line = line_number(source, offset);
            Some(format!("line {line}: pub(crate) fn {name}"))
        })
        .collect()
}

fn check_no_return_ok(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::RETURN_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let return_expr = ast::ReturnExpr::cast(syntax_node)?;
            let expr = return_expr.expr()?;
            let expr_str = expr.to_string();
            if expr_str != "Ok(())" {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: return Ok(())"))
        })
        .collect()
}

fn check_no_spawn(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    check_method_call(source, ast, "spawn")
}

fn check_no_sqlx_query_outside_models(source: &str, ast: &SourceFile, path: &str) -> Vec<String> {
    if is_in_models_dir(path) {
        return vec![];
    }

    let call_errors: Vec<String> = ast
        .syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let call = ast::CallExpr::cast(syntax_node)?;
            let ast::Expr::PathExpr(path_expr) = call.expr()? else {
                return None;
            };
            let query_path = path_expr.path()?;
            let path_text = query_path.to_string();
            let name = extract_sqlx_name(&path_text)?;
            if !is_sqlx_query_name(&name) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: sqlx::{name}(...)"))
        })
        .collect();

    let macro_errors: Vec<String> = ast
        .syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::MACRO_CALL {
                return None;
            }
            let syntax_node = node.clone();
            let macro_call = ast::MacroCall::cast(syntax_node)?;
            let macro_path = macro_call.path()?;
            let path_text = macro_path.to_string();
            let name = extract_sqlx_name(&path_text)?;
            if !is_sqlx_query_name(&name) {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: sqlx::{name}!"))
        })
        .collect();

    [call_errors, macro_errors].concat()
}

fn check_no_sqlx_query_without_tracing_instrument(
    source: &str,
    ast: &SourceFile,
    _path: &str,
) -> Vec<String> {
    collect_sqlx_query_fn_errors(source, ast, missing_tracing_instrument_error)
}

fn check_no_sqlx_query_without_skip_pool(
    source: &str,
    ast: &SourceFile,
    _path: &str,
) -> Vec<String> {
    collect_sqlx_query_fn_errors(source, ast, missing_pool_skip_error)
}

fn check_no_stderr_null(source: &str, ast: &SourceFile, _path: &str) -> Vec<String> {
    ast.syntax()
        .descendants()
        .filter_map(|node| {
            if node.kind() != SyntaxKind::METHOD_CALL_EXPR {
                return None;
            }
            let syntax_node = node.clone();
            let method_call = ast::MethodCallExpr::cast(syntax_node)?;
            let method_name = method_call.name_ref()?.text().to_string();
            if method_name != "stderr" {
                return None;
            }
            let arg_list = method_call.arg_list()?;
            let first_arg = arg_list.args().next()?;
            let arg_str = first_arg.to_string();
            if !arg_str.contains("null()") {
                return None;
            }
            let start = node.text_range().start();
            let offset: usize = start.into();
            let line = line_number(source, offset);
            Some(format!("line {line}: .stderr(Stdio::null())"))
        })
        .collect()
}

fn line_number(source: &str, offset: usize) -> usize {
    source.get(..offset).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
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

fn parse_cargo_tomls(paths: Vec<String>) -> (Vec<ParsedCargoToml>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<toml::Value>(&content) {
                Ok(toml) => parsed.push(ParsedCargoToml { path, toml }),
                Err(err) => errors.push(AppError::Parse {
                    file: path,
                    message: err.to_string(),
                }),
            },
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn run_rules(files: &[ParsedFile], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source, &file.ast, &file.path);
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

fn run_cargo_toml_rules(files: &[ParsedCargoToml], rules: &[CargoTomlRule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.toml);
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
    let (cargo_toml_paths, cargo_toml_glob_errors) = find(CARGO_TOML_PATTERNS, EXCLUDES);
    errors.extend(cargo_toml_glob_errors);

    let (parsed, parse_errors) = parse(paths);
    let (parsed_cargo_tomls, cargo_toml_errors) = parse_cargo_tomls(cargo_toml_paths);
    errors.extend(parse_errors);
    errors.extend(cargo_toml_errors);

    let mut violations = run_rules(&parsed, RULES);
    let cargo_violations = run_cargo_toml_rules(&parsed_cargo_tomls, CARGO_TOML_RULES);
    violations.extend(cargo_violations);
    CheckResult {
        errors,
        file_count: parsed.len() + parsed_cargo_tomls.len(),
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

    fn read_toml_fixture(name: &str) -> toml::Value {
        let path = format!("fixtures/rust/{name}.toml");
        let content = fs::read_to_string(&path).expect("fixture should exist");
        toml::from_str(&content).expect("valid toml")
    }

    #[test]
    fn pinned_versions_detects_violation() {
        let toml = read_toml_fixture("pinned-versions");
        let errors = check_pinned_versions(&toml);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn pinned_versions_allows_valid() {
        let toml = read_toml_fixture("valid-pinned-versions");
        let errors = check_pinned_versions(&toml);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_allow_attr_detects_violation() {
        let (source, ast) = parse_fixture("no-allow-attr");
        let errors = check_no_allow_attr(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_allow_attr_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_allow_attr(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_catch_unwind_detects_violation() {
        let (source, ast) = parse_fixture("no-catch-unwind");
        let errors = check_no_catch_unwind(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_catch_unwind_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-catch-unwind");
        let errors = check_no_catch_unwind(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_cfg_attr_suppress_detects_violation() {
        let (source, ast) = parse_fixture("no-cfg-attr-suppress");
        let errors = check_no_cfg_attr_suppress(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_cfg_attr_suppress_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_cfg_attr_suppress(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_discard_result_detects_violation() {
        let (source, ast) = parse_fixture("no-discard-result");
        let errors = check_no_discard_result(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_discard_result_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-discard-result");
        let errors = check_no_discard_result(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_discard_try_value_detects_violation() {
        let (source, ast) = parse_fixture("no-discard-try-value");
        let errors = check_no_discard_try_value(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_discard_try_value_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-discard-try-value");
        let errors = check_no_discard_try_value(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_drop_result_detects_violation() {
        let (source, ast) = parse_fixture("no-drop-result");
        let errors = check_no_drop_result(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_drop_result_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-drop-result");
        let errors = check_no_drop_result(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_direct_env_access_detects_violation() {
        let (source, ast) = parse_fixture("no-direct-env-access");
        let errors = check_no_direct_env_access(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_direct_env_access_allows_config_file() {
        let (source, ast) = parse_fixture("valid-no-direct-env-access");
        let errors = check_no_direct_env_access(&source, &ast, "src/config.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_direct_env_access_allows_env_file() {
        let (source, ast) = parse_fixture("valid-no-direct-env-access");
        let errors = check_no_direct_env_access(&source, &ast, "src/env.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_direct_env_access_allows_build_file() {
        let (source, ast) = parse_fixture("valid-no-direct-env-access");
        let errors = check_no_direct_env_access(&source, &ast, "build.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_expect_attr_detects_violation() {
        let (source, ast) = parse_fixture("no-expect-attr");
        let errors = check_no_expect_attr(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_expect_attr_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-attrs");
        let errors = check_no_expect_attr(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_nested_call_args_detects_violation() {
        let (source, ast) = parse_fixture("no-nested-call-args");
        let errors = check_no_nested_call_args(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_nested_call_args_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-nested-call-args");
        let errors = check_no_nested_call_args(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_expect_outside_tests_detects_violation() {
        let (source, ast) = parse_fixture("no-expect-outside-tests");
        let errors = check_no_expect_outside_tests(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_expect_outside_tests_allows_valid() {
        let (source, ast) = parse_fixture("valid-expect-in-tests");
        let errors = check_no_expect_outside_tests(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_expect_outside_tests_allows_integration_tests() {
        let (source, ast) = parse_fixture("no-expect-outside-tests");
        let errors = check_no_expect_outside_tests(&source, &ast, "tests/integration.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_if_let_ok_without_else_detects_violation() {
        let (source, ast) = parse_fixture("no-if-let-ok-without-else");
        let errors = check_no_if_let_ok_without_else(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_if_let_ok_without_else_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-if-let-ok-without-else");
        let errors = check_no_if_let_ok_without_else(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_let_ok_else_continue_detects_violation() {
        let (source, ast) = parse_fixture("no-let-ok-else-continue");
        let errors = check_no_let_ok_else_continue(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_let_ok_else_continue_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-let-ok-else-continue");
        let errors = check_no_let_ok_else_continue(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ignore_test_detects_violation() {
        let (source, ast) = parse_fixture("no-ignore-test");
        let errors = check_no_ignore_test(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ignore_test_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-ignore-test");
        let errors = check_no_ignore_test(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_impl_detects_violation() {
        let (source, ast) = parse_fixture("no-impl");
        let errors = check_no_impl(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_is_ok_is_err_detects_violation() {
        let (source, ast) = parse_fixture("no-is-ok-is-err");
        let errors = check_no_is_ok_is_err(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_is_ok_is_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-is-ok-is-err");
        let errors = check_no_is_ok_is_err(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_map_discard_detects_violation() {
        let (source, ast) = parse_fixture("no-map-discard");
        let errors = check_no_map_discard(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_map_discard_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-map-discard");
        let errors = check_no_map_discard(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_map_err_discard_detects_violation() {
        let (source, ast) = parse_fixture("no-map-err-discard");
        let errors = check_no_map_err_discard(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_map_err_discard_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-map-err-discard");
        let errors = check_no_map_err_discard(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_map_or_else_detects_violation() {
        let (source, ast) = parse_fixture("no-map-or-else");
        let errors = check_no_map_or_else(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_map_or_else_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-map-or-else");
        let errors = check_no_map_or_else(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_mem_forget_detects_violation() {
        let (source, ast) = parse_fixture("no-mem-forget");
        let errors = check_no_mem_forget(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_mem_forget_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-mem-forget");
        let errors = check_no_mem_forget(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ok_and_propagate_detects_violation() {
        let (source, ast) = parse_fixture("no-ok-and-propagate");
        let errors = check_no_ok_and_propagate(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ok_and_propagate_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-ok-and-propagate");
        let errors = check_no_ok_and_propagate(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_ok_method_detects_violation() {
        let (source, ast) = parse_fixture("no-ok-method");
        let errors = check_no_ok_method(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_ok_method_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-ok-method");
        let errors = check_no_ok_method(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_trait_impl_detects_violation() {
        let (source, ast) = parse_fixture("no-trait-impl");
        let errors = check_no_trait_impl(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_print_outside_main_detects_violation() {
        let (source, ast) = parse_fixture("no-print-outside-main");
        let errors = check_no_print_outside_main(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_print_outside_main_allows_valid() {
        let (source, ast) = parse_fixture("valid-print-in-main");
        let errors = check_no_print_outside_main(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_return_ok_detects_violation() {
        let (source, ast) = parse_fixture("no-return-ok");
        let errors = check_no_return_ok(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_return_ok_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-return-ok");
        let errors = check_no_return_ok(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_spawn_detects_violation() {
        let (source, ast) = parse_fixture("no-spawn");
        let errors = check_no_spawn(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_spawn_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-spawn");
        let errors = check_no_spawn(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_sqlx_query_outside_models_detects_violation() {
        let (source, ast) = parse_fixture("no-sqlx-query-outside-models");
        let errors = check_no_sqlx_query_outside_models(&source, &ast, "src/server/health.rs");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_sqlx_query_outside_models_allows_models_dir() {
        let (source, ast) = parse_fixture("no-sqlx-query-outside-models");
        let errors = check_no_sqlx_query_outside_models(&source, &ast, "src/models/user.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_sqlx_query_outside_models_does_not_allow_models_rs() {
        let (source, ast) = parse_fixture("no-sqlx-query-outside-models");
        let errors = check_no_sqlx_query_outside_models(&source, &ast, "src/models.rs");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_sqlx_query_outside_models_allows_non_query_sqlx_usage() {
        let (source, ast) = parse_fixture("valid-no-sqlx-query-outside-models");
        let errors = check_no_sqlx_query_outside_models(&source, &ast, "src/server/health.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_sqlx_query_without_tracing_instrument_detects_violation() {
        let (source, ast) = parse_fixture("no-sqlx-query-without-tracing-instrument");
        let errors =
            check_no_sqlx_query_without_tracing_instrument(&source, &ast, "src/models/user.rs");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_sqlx_query_without_tracing_instrument_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-sqlx-query-without-tracing-instrument");
        let errors =
            check_no_sqlx_query_without_tracing_instrument(&source, &ast, "src/models/user.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_sqlx_query_without_skip_pool_detects_violation() {
        let (source, ast) = parse_fixture("no-sqlx-query-without-skip-pool");
        let errors = check_no_sqlx_query_without_skip_pool(&source, &ast, "src/models/user.rs");
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_sqlx_query_without_skip_pool_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-sqlx-query-without-skip-pool");
        let errors = check_no_sqlx_query_without_skip_pool(&source, &ast, "src/models/user.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_stderr_null_detects_violation() {
        let (source, ast) = parse_fixture("no-stderr-null");
        let errors = check_no_stderr_null(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_stderr_null_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-stderr-null");
        let errors = check_no_stderr_null(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_err_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-err");
        let errors = check_no_unwrap_err(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_err(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or");
        let errors = check_no_unwrap_or(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_default_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or-default");
        let errors = check_no_unwrap_or_default(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_default_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or_default(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_or_else_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-or-else");
        let errors = check_no_unwrap_or_else(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_or_else_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_or_else(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unwrap_unchecked_detects_violation() {
        let (source, ast) = parse_fixture("no-unwrap-unchecked");
        let errors = check_no_unwrap_unchecked(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unwrap_unchecked_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unwrap-methods");
        let errors = check_no_unwrap_unchecked(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_unit_error_detects_violation() {
        let (source, ast) = parse_fixture("no-unit-error");
        let errors = check_no_unit_error(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_unit_error_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-unit-error");
        let errors = check_no_unit_error(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_while_let_ok_detects_violation() {
        let (source, ast) = parse_fixture("no-while-let-ok");
        let errors = check_no_while_let_ok(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_while_let_ok_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-while-let-ok");
        let errors = check_no_while_let_ok(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_swallowed_err_detects_violation() {
        let (source, ast) = parse_fixture("no-swallowed-err");
        let errors = check_no_swallowed_err(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_swallowed_err_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-swallowed-err");
        let errors = check_no_swallowed_err(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_inline_style_detects_violation() {
        let (source, ast) = parse_fixture("no-inline-style");
        let errors = check_no_inline_style(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_inline_style_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-inline-style");
        let errors = check_no_inline_style(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn no_pub_crate_fn_detects_violation() {
        let (source, ast) = parse_fixture("no-pub-crate-fn");
        let errors = check_no_pub_crate_fn(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_pub_crate_fn_allows_valid() {
        let (source, ast) = parse_fixture("valid-no-pub-crate-fn");
        let errors = check_no_pub_crate_fn(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }

    #[test]
    fn thiserror_unused_fields_detects_violation() {
        let (source, ast) = parse_fixture("thiserror-unused-fields");
        let errors = check_thiserror_unused_fields(&source, &ast, "src/main.rs");
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn thiserror_unused_fields_allows_valid() {
        let (source, ast) = parse_fixture("valid-thiserror-unused-fields");
        let errors = check_thiserror_unused_fields(&source, &ast, "src/main.rs");
        assert!(errors.is_empty());
    }
}
