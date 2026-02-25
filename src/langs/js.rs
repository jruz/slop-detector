use std::fs;
use std::path::Path;

use oxc::allocator::Allocator;
use oxc::ast::AstKind;
use oxc::ast::ast::{BindingPattern, Expression, LogicalOperator, TSType, TSTypeName};
use oxc::parser::Parser;
use oxc::span::SourceType;
use oxc_ast_visit::{Visit, walk};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["fixtures"];
const PACKAGE_JSON_PATTERNS: &[&str] = &["package.json"];
const TS_PATTERNS: &[&str] = &["*.ts", "*.tsx"];

const THROWING_FNS: &[&str] = &[
    "accessSync",
    "appendFileSync",
    "copyFileSync",
    "execFileSync",
    "execSync",
    "JSON.parse",
    "lstatSync",
    "mkdirSync",
    "readFileSync",
    "readdirSync",
    "readlinkSync",
    "realpathSync",
    "renameSync",
    "rmSync",
    "rmdirSync",
    "statSync",
    "unlinkSync",
    "writeFileSync",
];

struct PackageJsonRule {
    check: fn(pkg: &serde_json::Value) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const PACKAGE_JSON_RULES: &[PackageJsonRule] = &[
    PackageJsonRule {
        check: check_no_scripts_section,
        hint: None,
        name: "no-scripts-section",
    },
    PackageJsonRule {
        check: check_exact_versions,
        hint: None,
        name: "exact-versions",
    },
];

struct ParsedPackageJson {
    path: String,
    pkg: serde_json::Value,
}

fn check_no_scripts_section(pkg: &serde_json::Value) -> Vec<String> {
    match pkg.get("scripts").and_then(|scripts| scripts.as_object()) {
        Some(obj) if !obj.is_empty() => vec!["contains a scripts section".to_string()],
        _ => vec![],
    }
}

fn check_exact_versions(pkg: &serde_json::Value) -> Vec<String> {
    let dep_sections = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];

    let mut errors = Vec::new();

    for section in dep_sections {
        if let Some(deps) = pkg.get(section).and_then(|val| val.as_object()) {
            for (name, version) in deps {
                if let Some(ver) = version.as_str() {
                    let is_unpinned =
                        ver.starts_with('^') || ver.starts_with('~') || ver == "latest";
                    if is_unpinned {
                        errors.push(format!("{section}.{name}: \"{ver}\" not pinned"));
                    }
                }
            }
        }
    }

    errors
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

fn parse_package_jsons(paths: Vec<String>) -> (Vec<ParsedPackageJson>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(pkg) => parsed.push(ParsedPackageJson { path, pkg }),
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

fn run_package_json_rules(
    files: &[ParsedPackageJson],
    rules: &[PackageJsonRule],
) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.pkg);
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

fn line_number(source: &str, offset: u32) -> usize {
    let Ok(offset_usize) = usize::try_from(offset) else {
        return 1;
    };
    source.get(..offset_usize).map_or(1, |slice| {
        slice.chars().filter(|ch| *ch == '\n').count() + 1
    })
}

fn is_silent_default(expr: &Expression<'_>) -> bool {
    match expr {
        Expression::ObjectExpression(obj) => obj.properties.is_empty(),
        Expression::ArrayExpression(arr) => arr.elements.is_empty(),
        Expression::StringLiteral(str_lit) => str_lit.value.is_empty(),
        Expression::NumericLiteral(num_lit) => num_lit.value == 0.0,
        Expression::BooleanLiteral(bool_lit) => !bool_lit.value,
        _ => false,
    }
}

fn is_binding_named_main(pattern: &BindingPattern<'_>) -> bool {
    pattern
        .get_binding_identifier()
        .is_some_and(|id| id.name == "main")
}

fn get_callee_text(expr: &oxc::ast::ast::CallExpression<'_>) -> String {
    match &expr.callee {
        Expression::StaticMemberExpression(member) => {
            let object = match &member.object {
                Expression::Identifier(id) => id.name.to_string(),
                Expression::StaticMemberExpression(inner) => {
                    let inner_obj = match &inner.object {
                        Expression::Identifier(id) => id.name.to_string(),
                        _ => String::new(),
                    };
                    format!("{}.{}", inner_obj, inner.property.name)
                }
                _ => String::new(),
            };
            format!("{object}.{}", member.property.name)
        }
        _ => String::new(),
    }
}

struct TsVisitor<'src> {
    array_as_const_errors: Vec<String>,
    console_errors: Vec<String>,
    for_loop_errors: Vec<String>,
    inside_main: bool,
    inside_try: u32,
    interface_errors: Vec<String>,
    path: &'src str,
    throwing_fn_errors: Vec<String>,
    silent_default_errors: Vec<String>,
    source: &'src str,
    stderr_write_errors: Vec<String>,
    stdout_write_errors: Vec<String>,
}

impl<'src> Visit<'src> for TsVisitor<'src> {
    fn enter_node(&mut self, kind: AstKind<'src>) {
        if let AstKind::Function(func) = kind
            && let Some(id) = &func.id
            && id.name == "main"
        {
            self.inside_main = true;
        }

        if let AstKind::VariableDeclarator(decl) = kind
            && is_binding_named_main(&decl.id)
        {
            self.inside_main = true;
        }

        if matches!(kind, AstKind::TryStatement(_)) {
            self.inside_try += 1;
        }
    }

    fn leave_node(&mut self, kind: AstKind<'src>) {
        if let AstKind::Function(func) = kind
            && let Some(id) = &func.id
            && id.name == "main"
        {
            self.inside_main = false;
        }

        if let AstKind::VariableDeclarator(decl) = kind
            && is_binding_named_main(&decl.id)
        {
            self.inside_main = false;
        }

        if matches!(kind, AstKind::TryStatement(_)) {
            self.inside_try -= 1;
        }
    }

    fn visit_call_expression(&mut self, expr: &oxc::ast::ast::CallExpression<'src>) {
        let callee_text = get_callee_text(expr);

        let is_logging_file = self.path.ends_with("/log.ts") || self.path.ends_with("/tracing.ts");
        if callee_text.starts_with("console.") && !self.inside_main && !is_logging_file {
            let line = line_number(self.source, expr.span.start);
            self.console_errors
                .push(format!("line {line}: {callee_text}"));
        }

        if callee_text == "process.stderr.write" {
            let line = line_number(self.source, expr.span.start);
            self.stderr_write_errors.push(format!("line {line}"));
        }

        if callee_text == "process.stdout.write" {
            let line = line_number(self.source, expr.span.start);
            self.stdout_write_errors.push(format!("line {line}"));
        }

        let fn_name = match &expr.callee {
            Expression::Identifier(id) if THROWING_FNS.contains(&id.name.as_str()) => {
                Some(id.name.as_str())
            }
            _ if THROWING_FNS.contains(&callee_text.as_str()) => Some(callee_text.as_str()),
            _ => None,
        };
        if let Some(name) = fn_name
            && self.inside_try == 0
        {
            let line = line_number(self.source, expr.span.start);
            self.throwing_fn_errors.push(format!("line {line}: {name}"));
        }

        walk::walk_call_expression(self, expr);
    }

    fn visit_logical_expression(&mut self, expr: &oxc::ast::ast::LogicalExpression<'src>) {
        if matches!(
            expr.operator,
            LogicalOperator::Or | LogicalOperator::Coalesce
        ) && is_silent_default(&expr.right)
            && let Ok(start) = usize::try_from(expr.span.start)
            && let Ok(end) = usize::try_from(expr.span.end)
            && let Some(text) = self.source.get(start..end)
        {
            let line = line_number(self.source, expr.span.start);
            self.silent_default_errors
                .push(format!("line {line}: {text}"));
        }
        walk::walk_logical_expression(self, expr);
    }

    fn visit_ts_as_expression(&mut self, expr: &oxc::ast::ast::TSAsExpression<'src>) {
        if matches!(expr.expression, Expression::ArrayExpression(_))
            && matches!(
                &expr.type_annotation,
                TSType::TSTypeReference(type_ref)
                    if matches!(&type_ref.type_name, TSTypeName::IdentifierReference(id) if id.name == "const")
            )
        {
            let line = line_number(self.source, expr.span.start);
            self.array_as_const_errors.push(format!("line {line}"));
        }
        walk::walk_ts_as_expression(self, expr);
    }

    fn visit_for_statement(&mut self, stmt: &oxc::ast::ast::ForStatement<'src>) {
        let line = line_number(self.source, stmt.span.start);
        self.for_loop_errors.push(format!("line {line}: for"));
        walk::walk_for_statement(self, stmt);
    }

    fn visit_for_in_statement(&mut self, stmt: &oxc::ast::ast::ForInStatement<'src>) {
        let line = line_number(self.source, stmt.span.start);
        self.for_loop_errors.push(format!("line {line}: for...in"));
        walk::walk_for_in_statement(self, stmt);
    }

    fn visit_for_of_statement(&mut self, stmt: &oxc::ast::ast::ForOfStatement<'src>) {
        let line = line_number(self.source, stmt.span.start);
        self.for_loop_errors.push(format!("line {line}: for...of"));
        walk::walk_for_of_statement(self, stmt);
    }

    fn visit_ts_interface_declaration(
        &mut self,
        decl: &oxc::ast::ast::TSInterfaceDeclaration<'src>,
    ) {
        if decl.id.name != "Window" {
            let line = line_number(self.source, decl.span.start);
            let name = &decl.id.name;
            self.interface_errors
                .push(format!("line {line}: interface {name}"));
        }
        walk::walk_ts_interface_declaration(self, decl);
    }
}

fn collect_violations(visitor: TsVisitor<'_>) -> Vec<Violation> {
    let rules: &[(&str, Vec<String>, Option<&str>)] = &[
        (
            "no-array-as-const",
            visitor.array_as_const_errors,
            Some("declare the type instead"),
        ),
        (
            "no-for-loop",
            visitor.for_loop_errors,
            Some("use map, filter, or reduce instead"),
        ),
        (
            "no-throwing-fn-without-try",
            visitor.throwing_fn_errors,
            Some("wrap in try/catch"),
        ),
        (
            "no-interface",
            visitor.interface_errors,
            Some("use type instead"),
        ),
        (
            "no-silent-defaults",
            visitor.silent_default_errors,
            Some("use Result or Option instead of silent defaults"),
        ),
        (
            "no-console",
            visitor.console_errors,
            Some("use log.ts or tracing.ts"),
        ),
        (
            "no-stderr-write",
            visitor.stderr_write_errors,
            Some("use console.error instead"),
        ),
        (
            "no-stdout-write",
            visitor.stdout_write_errors,
            Some("use console.log instead"),
        ),
    ];

    rules
        .iter()
        .filter_map(|(name, errors, hint)| {
            if errors.is_empty() {
                return None;
            }
            Some(Violation {
                errors: errors.clone(),
                file: visitor.path.to_string(),
                hint: hint.map(str::to_string),
                rule: (*name).to_string(),
            })
        })
        .collect()
}

fn create_visitor<'src>(source: &'src str, path: &'src str) -> TsVisitor<'src> {
    TsVisitor {
        array_as_const_errors: Vec::new(),
        console_errors: Vec::new(),
        for_loop_errors: Vec::new(),
        inside_main: false,
        inside_try: 0,
        interface_errors: Vec::new(),
        path,
        throwing_fn_errors: Vec::new(),
        silent_default_errors: Vec::new(),
        source,
        stderr_write_errors: Vec::new(),
        stdout_write_errors: Vec::new(),
    }
}

fn check_ts_files(paths: Vec<String>) -> (Vec<Violation>, Vec<AppError>) {
    let mut all_violations = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => {
                let allocator = Allocator::default();
                let source_type = if Path::new(&path)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tsx"))
                {
                    SourceType::tsx()
                } else {
                    SourceType::ts()
                };

                let parser = Parser::new(&allocator, &source, source_type);
                let result = parser.parse();

                if result.panicked {
                    errors.push(AppError::Parse {
                        file: path,
                        message: "Parser panicked".to_string(),
                    });
                    continue;
                }

                let mut visitor = create_visitor(&source, &path);
                visitor.visit_program(&result.program);
                all_violations.extend(collect_violations(visitor));
            }
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (all_violations, errors)
}

pub fn check() -> CheckResult {
    let (package_json_paths, mut errors) = find(PACKAGE_JSON_PATTERNS, EXCLUDES);
    let (ts_paths, ts_glob_errors) = find(TS_PATTERNS, EXCLUDES);
    errors.extend(ts_glob_errors);

    let (parsed_packages, pkg_errors) = parse_package_jsons(package_json_paths);
    errors.extend(pkg_errors);
    let pkg_violations = run_package_json_rules(&parsed_packages, PACKAGE_JSON_RULES);

    let (ts_violations, ts_errors) = check_ts_files(ts_paths);
    errors.extend(ts_errors);

    let file_count = parsed_packages.len() + ts_violations.len();

    CheckResult {
        errors,
        file_count,
        violations: [pkg_violations, ts_violations].concat(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_fixture(name: &str) -> String {
        let path = format!("fixtures/js/{name}");
        fs::read_to_string(&path).expect("fixture should exist")
    }

    fn parse_ts_fixture(name: &str) -> Vec<Violation> {
        parse_ts_fixture_as(name, &format!("fixtures/js/{name}.ts"))
    }

    fn parse_ts_fixture_as(name: &str, path: &str) -> Vec<Violation> {
        let source = read_fixture(&format!("{name}.ts"));
        let allocator = Allocator::default();
        let parser = Parser::new(&allocator, &source, SourceType::ts());
        let result = parser.parse();
        let mut visitor = create_visitor(&source, path);
        visitor.visit_program(&result.program);
        collect_violations(visitor)
    }

    fn has_violation(violations: &[Violation], rule: &str) -> bool {
        violations.iter().any(|violation| violation.rule == rule)
    }

    #[test]
    fn no_array_as_const_detects_violation() {
        let violations = parse_ts_fixture("no-array-as-const");
        assert!(has_violation(&violations, "no-array-as-const"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_array_as_const_allows_valid() {
        let violations = parse_ts_fixture("valid-array-as-const");
        assert!(!has_violation(&violations, "no-array-as-const"));
    }

    #[test]
    fn exact_versions_detects_violation() {
        let content = read_fixture("exact-versions.json");
        let pkg: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        let errors = check_exact_versions(&pkg);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn exact_versions_allows_valid() {
        let content = read_fixture("valid-exact-versions.json");
        let pkg: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        let errors = check_exact_versions(&pkg);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_throwing_fn_without_try_detects_violation() {
        let violations = parse_ts_fixture("no-throwing-fn-without-try");
        assert!(has_violation(&violations, "no-throwing-fn-without-try"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_throwing_fn_without_try_allows_valid() {
        let violations = parse_ts_fixture("valid-throwing-fn-without-try");
        assert!(!has_violation(&violations, "no-throwing-fn-without-try"));
    }

    #[test]
    fn no_scripts_section_detects_violation() {
        let content = read_fixture("no-scripts-section.json");
        let pkg: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        let errors = check_no_scripts_section(&pkg);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn no_scripts_section_allows_valid() {
        let content = read_fixture("valid-no-scripts.json");
        let pkg: serde_json::Value = serde_json::from_str(&content).expect("valid json");
        let errors = check_no_scripts_section(&pkg);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_for_loop_detects_violation() {
        let violations = parse_ts_fixture("no-for-loop");
        assert!(has_violation(&violations, "no-for-loop"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_for_loop_allows_valid() {
        let violations = parse_ts_fixture("valid-for-loop");
        assert!(!has_violation(&violations, "no-for-loop"));
    }

    #[test]
    fn no_console_detects_violation() {
        let violations = parse_ts_fixture("no-console");
        assert!(has_violation(&violations, "no-console"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_console_allows_valid() {
        let violations = parse_ts_fixture("valid-console");
        assert!(!has_violation(&violations, "no-console"));
    }

    #[test]
    fn no_console_allows_log_ts() {
        let violations = parse_ts_fixture_as("no-console", "src/log.ts");
        assert!(!has_violation(&violations, "no-console"));
    }

    #[test]
    fn no_console_allows_tracing_ts() {
        let violations = parse_ts_fixture_as("no-console", "src/tracing.ts");
        assert!(!has_violation(&violations, "no-console"));
    }

    #[test]
    fn no_interface_detects_violation() {
        let violations = parse_ts_fixture("no-interface");
        assert!(has_violation(&violations, "no-interface"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_interface_allows_valid() {
        let violations = parse_ts_fixture("valid-type");
        assert!(!has_violation(&violations, "no-interface"));
    }

    #[test]
    fn no_interface_allows_window() {
        let violations = parse_ts_fixture("valid-interface-window");
        assert!(!has_violation(&violations, "no-interface"));
    }

    #[test]
    fn no_silent_defaults_detects_violation() {
        let violations = parse_ts_fixture("no-silent-defaults");
        assert!(has_violation(&violations, "no-silent-defaults"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_silent_defaults_allows_valid() {
        let violations = parse_ts_fixture("valid-result");
        assert!(!has_violation(&violations, "no-silent-defaults"));
    }

    #[test]
    fn no_stderr_write_detects_violation() {
        let violations = parse_ts_fixture("no-stderr-write");
        assert!(has_violation(&violations, "no-stderr-write"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_stdout_write_detects_violation() {
        let violations = parse_ts_fixture("no-stdout-write");
        assert!(has_violation(&violations, "no-stdout-write"));
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn no_stdout_write_allows_valid() {
        let violations = parse_ts_fixture("valid-stdout-write");
        assert!(!has_violation(&violations, "no-stdout-write"));
    }
}
