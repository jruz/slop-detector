use std::collections::HashSet;
use std::fs;
use std::path::Path;

use nu_cmd_extra::extra::add_extra_command_context;
use nu_cmd_lang::create_default_context;
use nu_command::add_shell_command_context;
use nu_parser::parse;
use nu_protocol::ast::{
    Block, Call, Comparison, Expr, Expression, ExternalArgument, Operator, PathMember,
    Traverse as _,
};
use nu_protocol::casing::Casing;
use nu_protocol::engine::{EngineState, StateWorkingSet};
use nu_protocol::{BlockId, Span, Value};
use nu_std::load_standard_library;

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["target", "node_modules"];
const PATTERNS: &[&str] = &["*.nu"];

struct Analysis<'ast> {
    base: usize,
    block: &'ast Block,
    source: &'ast str,
    working_set: &'ast StateWorkingSet<'ast>,
}

struct Rule {
    check: fn(&Analysis<'_>) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        check: check_no_catch_exit,
        hint: Some(
            "remove the `try`; an uncaught error already halts with a non-zero exit \
             and keeps the original span",
        ),
        name: "no-catch-exit",
    },
    Rule {
        check: check_no_complete_rethrow,
        hint: Some("drop `| complete`; a failing external command already raises an error"),
        name: "no-complete-rethrow",
    },
    Rule {
        check: check_no_config_file,
        hint: Some(
            "drop `--no-config-file`; non-interactive Nushell skips startup configuration by default",
        ),
        name: "no-config-file",
    },
    Rule {
        check: check_no_error_make,
        hint: Some("use `log error` with an actionable message, followed by `exit 1`"),
        name: "no-error-make",
    },
    Rule {
        check: check_no_external_in_each,
        hint: Some(
            "use a `for` loop; externals inside closures lose the exit code \
             and produce a noisy trace",
        ),
        name: "no-external-in-each",
    },
    Rule {
        check: check_no_ignore_errors,
        hint: Some("drop `--ignore-errors`; handle the failure or let it halt the script"),
        name: "no-ignore-errors",
    },
    Rule {
        check: check_no_print_stderr,
        hint: Some("use `log error`, `log warning` or `log info` from `use std/log`"),
        name: "no-print-stderr",
    },
    Rule {
        check: check_no_reduce,
        hint: Some("use a `for` loop with a `mut` variable"),
        name: "no-reduce",
    },
];

const EXEMPT_FILES: &[(&str, &[&str])] = &[("no-print-stderr", &["log.nu", "sandbox.nu"])];

fn line_in(source: &str, base: usize, span: Span) -> usize {
    let offset = span.start.saturating_sub(base);
    source
        .get(..offset)
        .map_or(1, |head| head.matches('\n').count() + 1)
}

fn line_of(analysis: &Analysis<'_>, span: Span) -> usize {
    line_in(analysis.source, analysis.base, span)
}

fn call_if_named<'ast>(
    working_set: &StateWorkingSet<'_>,
    name: &str,
    expr: &'ast Expression,
) -> Vec<&'ast Call> {
    match &expr.expr {
        Expr::Call(call) if working_set.get_decl(call.decl_id).name() == name => {
            vec![call.as_ref()]
        }
        _ => vec![],
    }
}

fn calls_in<'ast>(
    working_set: &'ast StateWorkingSet<'_>,
    block: &'ast Block,
    name: &str,
) -> Vec<&'ast Call> {
    let mut found = Vec::new();
    block.flat_map(
        working_set,
        &|expr| call_if_named(working_set, name, expr),
        &mut found,
    );
    found
}

fn block_of<'ast>(
    working_set: &'ast StateWorkingSet<'_>,
    expr: &Expression,
) -> Option<&'ast Block> {
    match &expr.expr {
        Expr::Block(id) | Expr::Closure(id) => {
            let block = working_set.get_block(*id);
            let inner = block.as_ref();
            Some(inner)
        }
        Expr::Keyword(keyword) => block_of(working_set, &keyword.expr),
        _ => None,
    }
}

fn catch_swallows(working_set: &StateWorkingSet<'_>, catch: &Expression) -> bool {
    block_of(working_set, catch).is_some_and(|block| {
        block.pipelines.is_empty() || !calls_in(working_set, block, "exit").is_empty()
    })
}

fn try_swallows(working_set: &StateWorkingSet<'_>, call: &Call) -> bool {
    call.positional_iter()
        .nth(1)
        .is_none_or(|catch| catch_swallows(working_set, catch))
}

fn check_no_catch_exit(analysis: &Analysis<'_>) -> Vec<String> {
    let working_set = analysis.working_set;
    calls_in(working_set, analysis.block, "try")
        .into_iter()
        .filter(|call| try_swallows(working_set, call))
        .map(|call| {
            let line = line_of(analysis, call.head);
            format!("line {line}: `try` swallows the error")
        })
        .collect()
}

fn is_exit_code_path(expr: &Expression) -> bool {
    let Expr::FullCellPath(path) = &expr.expr else {
        return false;
    };
    let name = "exit_code".to_string();
    let exit_code = PathMember::test_string(name, false, Casing::Sensitive);
    path.tail.last() == Some(&exit_code)
}

fn is_exit_code_failure(condition: &Expression) -> bool {
    let Expr::BinaryOp(lhs, op, rhs) = &condition.expr else {
        return false;
    };
    matches!(
        op.expr,
        Expr::Operator(Operator::Comparison(Comparison::NotEqual))
    ) && matches!(rhs.expr, Expr::Int(0))
        && is_exit_code_path(lhs)
}

fn starts_with_error_make(working_set: &StateWorkingSet<'_>, block: &Block) -> bool {
    let first = block
        .pipelines
        .first()
        .and_then(|pipeline| pipeline.elements.first());
    first.is_some_and(|element| {
        matches!(
            &element.expr.expr,
            Expr::Call(call) if working_set.get_decl(call.decl_id).name() == "error make"
        )
    })
}

fn only_rethrows(working_set: &StateWorkingSet<'_>, call: &Call) -> bool {
    let condition_fails = call
        .positional_iter()
        .next()
        .is_some_and(is_exit_code_failure);
    let body_rethrows = call
        .positional_iter()
        .nth(1)
        .and_then(|body| block_of(working_set, body))
        .is_some_and(|block| starts_with_error_make(working_set, block));
    condition_fails && body_rethrows
}

fn check_no_complete_rethrow(analysis: &Analysis<'_>) -> Vec<String> {
    let working_set = analysis.working_set;
    calls_in(working_set, analysis.block, "if")
        .into_iter()
        .filter(|call| only_rethrows(working_set, call))
        .map(|call| {
            let line = line_of(analysis, call.head);
            format!("line {line}: `exit_code != 0` only rethrows")
        })
        .collect()
}

fn check_no_error_make(analysis: &Analysis<'_>) -> Vec<String> {
    calls_in(analysis.working_set, analysis.block, "error make")
        .into_iter()
        .map(|call| {
            let line = line_of(analysis, call.head);
            format!("line {line}: `error make` call")
        })
        .collect()
}

fn check_no_ignore_errors(analysis: &Analysis<'_>) -> Vec<String> {
    calls_in(analysis.working_set, analysis.block, "do")
        .into_iter()
        .filter(|call| call.get_named_arg("ignore-errors").is_some())
        .map(|call| {
            let line = line_of(analysis, call.head);
            format!("line {line}: `do --ignore-errors` swallows the error")
        })
        .collect()
}

fn check_no_reduce(analysis: &Analysis<'_>) -> Vec<String> {
    calls_in(analysis.working_set, analysis.block, "reduce")
        .into_iter()
        .map(|call| {
            let line = line_of(analysis, call.head);
            format!("line {line}: `reduce` hides state in an accumulator")
        })
        .collect()
}

struct External {
    args: Vec<String>,
    name: String,
    span: Span,
}

fn text_of(working_set: &StateWorkingSet<'_>, span: Span) -> String {
    let contents = working_set.get_span_contents(span);
    String::from_utf8_lossy(contents).into_owned()
}

fn external_of(working_set: &StateWorkingSet<'_>, expr: &Expression) -> Vec<External> {
    let Expr::ExternalCall(head, args) = &expr.expr else {
        return vec![];
    };
    let head_text = text_of(working_set, head.span);
    let name = head_text.trim_start_matches('^').to_string();
    let arg_texts = args
        .iter()
        .map(|arg| match arg {
            ExternalArgument::Regular(value) | ExternalArgument::Spread(value) => {
                text_of(working_set, value.span)
            }
        })
        .collect();
    vec![External {
        args: arg_texts,
        name,
        span: expr.span,
    }]
}

fn externals_in(working_set: &StateWorkingSet<'_>, block: &Block) -> Vec<External> {
    let mut found = Vec::new();
    block.flat_map(
        working_set,
        &|expr| external_of(working_set, expr),
        &mut found,
    );
    found
}

fn check_no_config_file(analysis: &Analysis<'_>) -> Vec<String> {
    externals_in(analysis.working_set, analysis.block)
        .into_iter()
        .filter(|external| {
            external.name == "nu" && external.args.iter().any(|arg| arg == "--no-config-file")
        })
        .map(|external| {
            let line = line_of(analysis, external.span);
            format!("line {line}: `nu --no-config-file` is unnecessary")
        })
        .collect()
}

fn is_print_to_stderr(external: &External) -> bool {
    external.name == "print"
        && external
            .args
            .iter()
            .any(|arg| arg == "--stderr" || arg == "-e")
}

fn check_no_print_stderr(analysis: &Analysis<'_>) -> Vec<String> {
    externals_in(analysis.working_set, analysis.block)
        .into_iter()
        .filter(is_print_to_stderr)
        .map(|external| {
            let line = line_of(analysis, external.span);
            format!("line {line}: `print --stderr` bypasses logging")
        })
        .collect()
}

fn external_calls_in(working_set: &StateWorkingSet<'_>, block: &Block) -> Vec<External> {
    externals_in(working_set, block)
        .into_iter()
        .filter(|external| external.name != "print")
        .collect()
}

fn local_function_blocks(working_set: &StateWorkingSet<'_>, block: &Block) -> Vec<BlockId> {
    let mut found = Vec::new();
    block.flat_map(
        working_set,
        &|expr| match &expr.expr {
            Expr::Call(call) => working_set
                .get_decl(call.decl_id)
                .block_id()
                .into_iter()
                .collect(),
            _ => vec![],
        },
        &mut found,
    );
    found
}

fn reaches_external(
    working_set: &StateWorkingSet<'_>,
    block: &Block,
    visited: &mut HashSet<BlockId>,
) -> bool {
    if !external_calls_in(working_set, block).is_empty() {
        return true;
    }
    let unvisited: Vec<BlockId> = local_function_blocks(working_set, block)
        .into_iter()
        .filter(|id| visited.insert(*id))
        .collect();
    unvisited.into_iter().any(|id| {
        let callee = working_set.get_block(id);
        reaches_external(working_set, callee, visited)
    })
}

fn closure_runs_external(working_set: &StateWorkingSet<'_>, call: &Call) -> bool {
    call.positional_iter()
        .filter_map(|arg| block_of(working_set, arg))
        .any(|block| {
            let mut visited = HashSet::new();
            reaches_external(working_set, block, &mut visited)
        })
}

fn check_no_external_in_each(analysis: &Analysis<'_>) -> Vec<String> {
    let working_set = analysis.working_set;
    ["each", "par-each", "items"]
        .into_iter()
        .flat_map(|name| calls_in(working_set, analysis.block, name))
        .filter(|call| closure_runs_external(working_set, call))
        .map(|call| {
            let line = line_of(analysis, call.head);
            let name = working_set.get_decl(call.decl_id).name();
            format!("line {line}: external command runs inside `{name}`")
        })
        .collect()
}

fn engine_state_for(path: &str) -> Result<EngineState, AppError> {
    let absolute = std::path::absolute(path).map_err(|err| AppError::Read {
        file: path.to_string(),
        message: err.to_string(),
    })?;
    let directory = absolute
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .ok_or_else(|| AppError::Read {
            file: path.to_string(),
            message: "file has no parent directory".to_string(),
        })?;
    let language_commands = create_default_context();
    let shell_commands = add_shell_command_context(language_commands);
    let mut engine_state = add_extra_command_context(shell_commands);
    load_standard_library(&mut engine_state).map_err(|err| AppError::Parse {
        file: path.to_string(),
        message: err.to_string(),
    })?;
    engine_state.file = Some(absolute);
    let span = Span::unknown();
    for name in ["PWD", "FILE_PWD"] {
        let location = directory.clone();
        let value = Value::string(location, span);
        let key = name.to_string();
        engine_state.add_env_var(key, value);
    }
    Ok(engine_state)
}

fn with_analysis<T>(
    path: &str,
    source: &str,
    run: impl FnOnce(&Analysis<'_>) -> T,
) -> Result<T, AppError> {
    let engine_state = engine_state_for(path)?;
    let mut working_set = StateWorkingSet::new(&engine_state);
    let base = working_set.next_span_start();
    let contents = source.as_bytes();
    let file = Some(path);
    let block = parse(&mut working_set, file, contents, false);
    if !working_set.parse_errors.is_empty() {
        let message = working_set
            .parse_errors
            .iter()
            .map(|err| {
                let span = err.span();
                let line = line_in(source, base, span);
                format!("line {line}: {err}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        return Err(AppError::Parse {
            file: path.to_string(),
            message,
        });
    }
    let analysis = Analysis {
        base,
        block: block.as_ref(),
        source,
        working_set: &working_set,
    };
    let result = run(&analysis);
    Ok(result)
}

struct ParsedNu {
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

fn read(paths: Vec<String>) -> (Vec<ParsedNu>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => parsed.push(ParsedNu { path, source }),
            Err(err) => errors.push(AppError::Read {
                file: path,
                message: err.to_string(),
            }),
        }
    }

    (parsed, errors)
}

fn is_exempt(rule: &Rule, path: &str) -> bool {
    let file_name = Path::new(path).file_name().and_then(|name| name.to_str());
    EXEMPT_FILES
        .iter()
        .filter(|(name, _)| *name == rule.name)
        .any(|(_, files)| file_name.is_some_and(|file| files.contains(&file)))
}

fn violations_of(path: &str, analysis: &Analysis<'_>) -> Vec<Violation> {
    RULES
        .iter()
        .filter(|rule| !is_exempt(rule, path))
        .filter_map(|rule| {
            let errors = (rule.check)(analysis);
            if errors.is_empty() {
                return None;
            }
            Some(Violation {
                errors,
                file: path.to_string(),
                hint: rule.hint.map(String::from),
                rule: rule.name.to_string(),
            })
        })
        .collect()
}

fn run_rules(files: &[ParsedNu]) -> (Vec<Violation>, Vec<AppError>) {
    let mut violations = Vec::new();
    let mut errors = Vec::new();

    for file in files {
        let analyzed = with_analysis(&file.path, &file.source, |analysis| {
            violations_of(&file.path, analysis)
        });
        match analyzed {
            Ok(found) => violations.extend(found),
            Err(err) => errors.push(err),
        }
    }

    (violations, errors)
}

pub fn check() -> CheckResult {
    let (paths, mut errors) = find(PATTERNS, EXCLUDES);
    let (files, read_errors) = read(paths);
    errors.extend(read_errors);
    let (violations, parse_errors) = run_rules(&files);
    errors.extend(parse_errors);

    CheckResult {
        errors,
        file_count: files.len(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_fixture(name: &str, check: fn(&Analysis<'_>) -> Vec<String>) -> Vec<String> {
        let path = format!("fixtures/nu/{name}.nu");
        let source = fs::read_to_string(&path).expect("fixture should exist");
        with_analysis(&path, &source, check).expect("fixture should parse")
    }

    #[test]
    fn no_complete_rethrow_detects_violation() {
        let errors = run_fixture("no-complete-rethrow", check_no_complete_rethrow);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_complete_rethrow_allows_valid() {
        let errors = run_fixture("valid-no-complete-rethrow", check_no_complete_rethrow);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_catch_exit_detects_violation() {
        let errors = run_fixture("no-catch-exit", check_no_catch_exit);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_catch_exit_allows_valid() {
        let errors = run_fixture("valid-no-catch-exit", check_no_catch_exit);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_config_file_detects_violation() {
        let errors = run_fixture("no-config-file", check_no_config_file);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_config_file_allows_valid() {
        let errors = run_fixture("valid-no-config-file", check_no_config_file);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_error_make_detects_violation() {
        let errors = run_fixture("no-error-make", check_no_error_make);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_error_make_allows_valid() {
        let errors = run_fixture("valid-no-error-make", check_no_error_make);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_ignore_errors_detects_violation() {
        let errors = run_fixture("no-ignore-errors", check_no_ignore_errors);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_ignore_errors_allows_valid() {
        let errors = run_fixture("valid-no-ignore-errors", check_no_ignore_errors);
        assert_eq!(errors, Vec::<String>::new());
    }

    fn violated_rules_as(fixture: &str, path: &str) -> Vec<String> {
        let fixture_path = format!("fixtures/nu/{fixture}.nu");
        let source = fs::read_to_string(&fixture_path).expect("fixture should exist");
        let violations = with_analysis(&fixture_path, &source, |analysis| {
            violations_of(path, analysis)
        })
        .expect("fixture should parse");
        violations
            .into_iter()
            .map(|violation| violation.rule)
            .collect()
    }

    #[test]
    fn no_print_stderr_allows_log_nu() {
        let rules = violated_rules_as("no-print-stderr", "scripts/lib/log.nu");
        assert!(!rules.contains(&"no-print-stderr".to_string()));
    }

    #[test]
    fn no_print_stderr_allows_sandbox_nu() {
        let rules = violated_rules_as("no-print-stderr", "scripts/sandbox.nu");
        assert!(!rules.contains(&"no-print-stderr".to_string()));
    }

    #[test]
    fn no_print_stderr_flags_other_files() {
        let rules = violated_rules_as("no-print-stderr", "scripts/build.nu");
        assert!(rules.contains(&"no-print-stderr".to_string()));
    }

    #[test]
    fn no_print_stderr_detects_violation() {
        let errors = run_fixture("no-print-stderr", check_no_print_stderr);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_print_stderr_allows_valid() {
        let errors = run_fixture("valid-no-print-stderr", check_no_print_stderr);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_reduce_detects_violation() {
        let errors = run_fixture("no-reduce", check_no_reduce);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_reduce_allows_valid() {
        let errors = run_fixture("valid-no-reduce", check_no_reduce);
        assert_eq!(errors, Vec::<String>::new());
    }

    #[test]
    fn no_external_in_each_detects_violation() {
        let errors = run_fixture("no-external-in-each", check_no_external_in_each);
        assert_eq!(errors.len(), 4);
    }

    #[test]
    fn no_external_in_each_allows_valid() {
        let errors = run_fixture("valid-no-external-in-each", check_no_external_in_each);
        assert_eq!(errors, Vec::<String>::new());
    }
}
