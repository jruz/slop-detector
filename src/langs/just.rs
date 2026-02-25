use std::fs;

use tree_sitter::{Node, Parser, Tree};

use crate::files::find_files;
use crate::types::{AppError, CheckResult, Violation};

const EXCLUDES: &[&str] = &["build", "fixtures"];
const PATTERNS: &[&str] = &["justfile", "*.just"];

struct Rule {
    check: fn(source: &str, tree: &Tree) -> Vec<String>,
    hint: Option<&'static str>,
    name: &'static str,
}

const BANNED_SHEBANGS: &[&str] = &["bash", "sh"];

const KNOWN_EXTERNALS: &[&str] = &[
    "bun",
    "bunx",
    "cargo",
    "chmod",
    "claude",
    "deno",
    "direnv",
    "docker",
    "fd",
    "git",
    "gleam",
    "just",
    "npm",
    "npx",
    "postgres",
    "scp",
    "sh",
    "sops",
    "ssh",
    "ssh-keygen",
    "sqlx",
    "tail",
];

const RULES: &[Rule] = &[
    Rule {
        check: check_no_bash_shebang,
        hint: Some("use `#!/usr/bin/env nu` instead"),
        name: "no-bash-shebang",
    },
    Rule {
        check: check_no_external_without_caret,
        hint: Some("use `^command` to mark external commands in nu recipes"),
        name: "no-external-without-caret",
    },
    Rule {
        check: check_no_recipe_deps,
        hint: Some("call dependencies explicitly with `just <recipe>` in the body"),
        name: "no-recipe-deps",
    },
];

struct ParsedJust {
    path: String,
    source: String,
    tree: Tree,
}

fn create_parser() -> Result<Parser, tree_sitter::LanguageError> {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_just::language())?;
    Ok(parser)
}

fn find_recipes<'tree>(node: Node<'tree>, recipes: &mut Vec<Node<'tree>>) {
    if node.kind() == "recipe" {
        recipes.push(node);
    }
    for child in node.children(&mut node.walk()) {
        find_recipes(child, recipes);
    }
}

fn recipe_name<'tree>(
    recipe: Node<'tree>,
    source: &'tree [u8],
) -> Result<Option<&'tree str>, std::str::Utf8Error> {
    for child in recipe.children(&mut recipe.walk()) {
        if child.kind() == "recipe_header" {
            for header_child in child.children(&mut child.walk()) {
                if header_child.kind() == "identifier" {
                    return Ok(Some(header_child.utf8_text(source)?));
                }
            }
        }
    }
    Ok(None)
}

fn has_dependencies(recipe: Node<'_>) -> bool {
    for child in recipe.children(&mut recipe.walk()) {
        if child.kind() == "recipe_header" {
            for header_child in child.children(&mut child.walk()) {
                if header_child.kind() == "dependencies" {
                    return true;
                }
            }
        }
    }
    false
}

fn shebang_language<'tree>(
    recipe: Node<'tree>,
    source: &'tree [u8],
) -> Result<Option<(usize, &'tree str)>, std::str::Utf8Error> {
    for child in recipe.children(&mut recipe.walk()) {
        if child.kind() == "recipe_body" {
            for body_child in child.children(&mut child.walk()) {
                if body_child.kind() == "shebang" {
                    for shebang_child in body_child.children(&mut body_child.walk()) {
                        if shebang_child.kind() == "language" {
                            let line = body_child.start_position().row + 1;
                            let lang = shebang_child.utf8_text(source)?;
                            return Ok(Some((line, lang)));
                        }
                    }
                }
            }
        }
    }
    Ok(None)
}

fn is_nu_recipe(recipe: Node<'_>, source: &[u8]) -> bool {
    matches!(shebang_language(recipe, source), Ok(Some((_, "nu"))))
}

fn recipe_body_lines<'tree>(recipe: Node<'tree>, source: &'tree [u8]) -> Vec<(usize, &'tree str)> {
    let mut lines = Vec::new();
    for child in recipe.children(&mut recipe.walk()) {
        if child.kind() != "recipe_body" {
            continue;
        }
        for body_child in child.children(&mut child.walk()) {
            if body_child.kind() != "recipe_line" {
                continue;
            }
            let line_num = body_child.start_position().row + 1;
            for line_child in body_child.children(&mut body_child.walk()) {
                if line_child.kind() == "text"
                    && let Ok(text) = line_child.utf8_text(source)
                {
                    lines.push((line_num, text));
                }
            }
        }
    }
    lines
}

fn first_token(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let token = trimmed.split_whitespace().next()?;
    Some(token)
}

fn check_no_external_without_caret(source: &str, tree: &Tree) -> Vec<String> {
    let source_bytes = source.as_bytes();
    let mut recipes = Vec::new();
    find_recipes(tree.root_node(), &mut recipes);

    let mut errors = Vec::new();
    for recipe in &recipes {
        if !is_nu_recipe(*recipe, source_bytes) {
            continue;
        }
        for (line_num, text) in recipe_body_lines(*recipe, source_bytes) {
            let Some(token) = first_token(text) else {
                continue;
            };
            if KNOWN_EXTERNALS.contains(&token) {
                let name = recipe_name(*recipe, source_bytes);
                match name {
                    Ok(Some(rname)) => {
                        errors.push(format!(
                            "line {line_num}: recipe `{rname}` calls `{token}` without `^`"
                        ));
                    }
                    Ok(None) => {
                        errors.push(format!(
                            "line {line_num}: recipe calls `{token}` without `^`"
                        ));
                    }
                    Err(utf8_err) => {
                        errors.push(format!("UTF-8 error: {utf8_err}"));
                    }
                }
            }
        }
    }
    errors
}

fn check_no_bash_shebang(source: &str, tree: &Tree) -> Vec<String> {
    let source_bytes = source.as_bytes();
    let mut recipes = Vec::new();
    find_recipes(tree.root_node(), &mut recipes);

    let mut errors = Vec::new();
    for recipe in recipes {
        match shebang_language(recipe, source_bytes) {
            Ok(Some((line, lang))) if BANNED_SHEBANGS.contains(&lang) => {
                match recipe_name(recipe, source_bytes) {
                    Ok(Some(name)) => {
                        errors.push(format!("line {line}: recipe `{name}` uses {lang} shebang"));
                    }
                    Ok(None) => {
                        errors.push(format!("line {line}: recipe uses {lang} shebang"));
                    }
                    Err(utf8_err) => {
                        errors.push(format!("UTF-8 error: {utf8_err}"));
                    }
                }
            }
            Ok(_) => {}
            Err(utf8_err) => {
                errors.push(format!("UTF-8 error: {utf8_err}"));
            }
        }
    }
    errors
}

fn check_no_recipe_deps(source: &str, tree: &Tree) -> Vec<String> {
    let source_bytes = source.as_bytes();
    let mut recipes = Vec::new();
    find_recipes(tree.root_node(), &mut recipes);

    let mut errors = Vec::new();
    for recipe in recipes {
        if has_dependencies(recipe) {
            let line = recipe.start_position().row + 1;
            match recipe_name(recipe, source_bytes) {
                Ok(Some(name)) => {
                    errors.push(format!("line {line}: recipe `{name}` uses dependencies"));
                }
                Ok(None) => {
                    errors.push(format!("line {line}: recipe uses dependencies"));
                }
                Err(utf8_err) => {
                    errors.push(format!("UTF-8 error: {utf8_err}"));
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

fn parse(paths: Vec<String>, parser: &mut Parser) -> (Vec<ParsedJust>, Vec<AppError>) {
    let mut parsed = Vec::new();
    let mut errors = Vec::new();

    for path in paths {
        match fs::read_to_string(&path) {
            Ok(source) => match parser.parse(&source, None) {
                Some(tree) => parsed.push(ParsedJust { path, source, tree }),
                None => errors.push(AppError::Parse {
                    file: path,
                    message: "failed to parse".to_string(),
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

fn run_rules(files: &[ParsedJust], rules: &[Rule]) -> Vec<Violation> {
    files
        .iter()
        .flat_map(|file| {
            rules.iter().filter_map(|rule| {
                let errors = (rule.check)(&file.source, &file.tree);
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
    let (parsed, parse_errors) = match create_parser() {
        Ok(mut parser) => parse(paths, &mut parser),
        Err(err) => {
            errors.push(AppError::Parse {
                file: "just".to_string(),
                message: err.to_string(),
            });
            (Vec::new(), Vec::new())
        }
    };
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

    fn parse_fixture(name: &str) -> (String, Tree) {
        let path = format!("fixtures/just/{name}.just");
        let source = fs::read_to_string(&path).expect("fixture should exist");
        let mut parser = create_parser().expect("parser should load");
        let tree = parser.parse(&source, None).expect("should parse");
        (source, tree)
    }

    #[test]
    fn no_bash_shebang_detects_violation() {
        let (source, tree) = parse_fixture("no-bash-shebang");
        let errors = check_no_bash_shebang(&source, &tree);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_bash_shebang_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-bash-shebang");
        let errors = check_no_bash_shebang(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_external_without_caret_detects_violation() {
        let (source, tree) = parse_fixture("no-external-without-caret");
        let errors = check_no_external_without_caret(&source, &tree);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn no_external_without_caret_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-external-without-caret");
        let errors = check_no_external_without_caret(&source, &tree);
        assert!(errors.is_empty());
    }

    #[test]
    fn no_recipe_deps_detects_violation() {
        let (source, tree) = parse_fixture("no-recipe-deps");
        let errors = check_no_recipe_deps(&source, &tree);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn no_recipe_deps_allows_valid() {
        let (source, tree) = parse_fixture("valid-no-recipe-deps");
        let errors = check_no_recipe_deps(&source, &tree);
        assert!(errors.is_empty());
    }
}
