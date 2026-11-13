use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::{Node, Tree};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("line {line}: cannot statically render Just recipe: {message}")]
    Static { line: usize, message: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
}

pub type Parents = HashMap<PathBuf, Vec<PathBuf>>;

pub struct Document<'source> {
    pub path: &'source str,
    pub source: &'source str,
    pub tree: &'source Tree,
}

struct Context<'source> {
    document: &'source Document<'source>,
    root: &'source Path,
    source_path: &'source Path,
    parameters: HashSet<String>,
}

fn failure(node: Node<'_>, message: &str) -> Error {
    Error::Static {
        line: node.start_position().row + 1,
        message: message.to_string(),
    }
}

fn nodes_of<'tree>(node: Node<'tree>, kind: &str) -> Vec<Node<'tree>> {
    if node.kind() == kind {
        return vec![node];
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .flat_map(|child| nodes_of(child, kind))
        .collect()
}

fn text<'source>(node: Node<'_>, source: &'source str) -> Result<&'source str, Error> {
    let bytes = source.as_bytes();
    let contents = node.utf8_text(bytes)?;
    Ok(contents)
}

fn string_value(node: Node<'_>, source: &str) -> Result<String, Error> {
    let literal = text(node, source)?;
    if let Some(inner) = literal
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        let value = inner.to_string();
        return Ok(value);
    }
    if literal.starts_with('"') {
        return match serde_json::from_str(literal) {
            Ok(value) => Ok(value),
            Err(err) => {
                let message = format!("unsupported Just string escape: {err}");
                let error = failure(node, &message);
                Err(error)
            }
        };
    }
    let error = failure(node, "string requires runtime evaluation");
    Err(error)
}

fn parent(path: &Path) -> Result<&Path, Error> {
    path.parent().ok_or_else(|| Error::Static {
        line: 1,
        message: "Justfile has no parent directory".to_string(),
    })
}

fn builtin(node: Node<'_>, context: &Context<'_>) -> Result<String, Error> {
    let name_node = node
        .child_by_field_name("name")
        .ok_or_else(|| failure(node, "function has no name"))?;
    let name = text(name_node, context.document.source)?;
    if node.named_child_count() != 1 {
        let error = failure(node, "function arguments require unsupported evaluation");
        return Err(error);
    }
    let value = match name {
        "justfile_directory" => parent(context.root)?,
        "justfile" => context.root,
        "source_directory" => parent(context.source_path)?,
        "source_file" => context.source_path,
        _ => {
            let error = failure(node, "function requires runtime or unsupported evaluation");
            return Err(error);
        }
    };
    let rendered = value.to_string_lossy().into_owned();
    Ok(rendered)
}

fn variable(
    node: Node<'_>,
    context: &Context<'_>,
    active: &mut HashSet<String>,
) -> Result<String, Error> {
    let name = text(node, context.document.source)?;
    if context.parameters.contains(name) {
        let error = failure(node, "recipe parameter requires runtime arguments");
        return Err(error);
    }
    let key = name.to_string();
    if !active.insert(key) {
        let error = failure(node, "cyclic Just variable");
        return Err(error);
    }
    let root = context.document.tree.root_node();
    let bytes = context.document.source.as_bytes();
    let assignment = nodes_of(root, "assignment")
        .into_iter()
        .find(|assignment| {
            assignment.child_by_field_name("left").is_some_and(|left| {
                let contents = left.utf8_text(bytes);
                let expected = Ok(name);
                contents == expected
            })
        })
        .ok_or_else(|| failure(node, "variable is not a local static assignment"))?;
    let value = assignment
        .child_by_field_name("right")
        .ok_or_else(|| failure(node, "assignment has no value"))?;
    let result = evaluate(value, context, active)?;
    active.remove(name);
    Ok(result)
}

fn evaluate(
    node: Node<'_>,
    context: &Context<'_>,
    active: &mut HashSet<String>,
) -> Result<String, Error> {
    match node.kind() {
        "string" => string_value(node, context.document.source),
        "identifier" => variable(node, context, active),
        "function_call" => builtin(node, context),
        "expression" | "value" => {
            let mut result = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.is_named() || child.kind() == "expression" {
                    let value = evaluate(child, context, active)?;
                    result.push_str(&value);
                } else {
                    let operator = text(child, context.document.source)?;
                    match operator {
                        "+" | "(" | ")" => {}
                        "/" => result.push('/'),
                        _ => {
                            let message =
                                format!("unsupported Just expression token `{}`", child.kind());
                            let error = failure(child, &message);
                            return Err(error);
                        }
                    }
                }
            }
            Ok(result)
        }
        _ => {
            let error = failure(
                node,
                "expression requires runtime or unsupported evaluation",
            );
            Err(error)
        }
    }
}

fn referenced_files(document: &Document<'_>) -> Result<Vec<PathBuf>, Error> {
    let absolute = fs::canonicalize(document.path)?;
    let directory = parent(&absolute)?;
    let root = document.tree.root_node();
    let mut references = Vec::new();
    for kind in ["module", "import"] {
        for node in nodes_of(root, kind) {
            let strings = nodes_of(node, "string");
            let candidates = if let Some(path_node) = strings.first() {
                let path = string_value(*path_node, document.source)?;
                if path.starts_with("~/") {
                    let error = failure(
                        *path_node,
                        "home-relative Just references require a runtime environment",
                    );
                    return Err(error);
                }
                vec![directory.join(path)]
            } else {
                let name_node = node
                    .child_by_field_name("name")
                    .ok_or_else(|| failure(node, "module has no name"))?;
                let name = text(name_node, document.source)?;
                [
                    format!("{name}.just"),
                    format!("{name}/mod.just"),
                    format!("{name}/justfile"),
                    format!("{name}/.justfile"),
                ]
                .into_iter()
                .map(|path| directory.join(path))
                .collect()
            };
            for path in candidates {
                if path.try_exists()? {
                    let canonical = fs::canonicalize(path)?;
                    references.push(canonical);
                    break;
                }
            }
        }
    }
    Ok(references)
}

pub fn parents(documents: &[Document<'_>]) -> Result<Parents, Error> {
    let mut graph = HashMap::new();
    for document in documents {
        let path = fs::canonicalize(document.path)?;
        for child in referenced_files(document)? {
            let ancestor = path.clone();
            graph.entry(child).or_insert_with(Vec::new).push(ancestor);
        }
    }
    Ok(graph)
}

fn roots(
    path: &Path,
    graph: &Parents,
    active: &mut HashSet<PathBuf>,
) -> Result<Vec<PathBuf>, Error> {
    let key = path.to_path_buf();
    if !active.insert(key) {
        return Err(Error::Static {
            line: 1,
            message: "cyclic Just imports or modules".to_string(),
        });
    }
    let mut found = Vec::new();
    match graph.get(path) {
        None => {
            let root = path.to_path_buf();
            found.push(root);
        }
        Some(parents) => {
            for ancestor in parents {
                let ancestor_roots = roots(ancestor, graph, active)?;
                found.extend(ancestor_roots);
            }
        }
    }
    active.remove(path);
    found.sort();
    found.dedup();
    Ok(found)
}

fn render_body(body: Node<'_>, context: &Context<'_>) -> Result<String, Error> {
    let source = context.document.source;
    let mut script = "\n".repeat(body.start_position().row);
    let mut offset = body.start_byte();
    for interpolation in nodes_of(body, "interpolation") {
        if interpolation.start_position().row != interpolation.end_position().row {
            let error = failure(interpolation, "multiline interpolation is not supported");
            return Err(error);
        }
        let range = offset..interpolation.start_byte();
        let preceding = source
            .get(range)
            .ok_or_else(|| failure(body, "invalid recipe span"))?;
        script.push_str(preceding);
        let expression = interpolation
            .named_child(0)
            .ok_or_else(|| failure(interpolation, "interpolation has no expression"))?;
        let mut active = HashSet::new();
        let value = evaluate(expression, context, &mut active)?;
        if value.contains(['\n', '\r']) {
            let error = failure(interpolation, "multiline interpolation is not supported");
            return Err(error);
        }
        script.push_str(&value);
        offset = interpolation.end_byte();
    }
    let range = offset..body.end_byte();
    let remaining = source
        .get(range)
        .ok_or_else(|| failure(body, "invalid recipe span"))?;
    script.push_str(remaining);
    Ok(script)
}

pub fn render(
    document: &Document<'_>,
    recipe: Node<'_>,
    graph: &Parents,
) -> Result<Vec<String>, Error> {
    let body = nodes_of(recipe, "recipe_body")
        .into_iter()
        .next()
        .ok_or_else(|| failure(recipe, "Nushell recipe has no body"))?;
    let parameters: HashSet<String> = nodes_of(recipe, "parameter")
        .into_iter()
        .filter_map(|parameter| parameter.child_by_field_name("name"))
        .map(|name| text(name, document.source).map(String::from))
        .collect::<Result<_, _>>()?;
    let absolute = fs::canonicalize(document.path)?;
    let mut active = HashSet::new();
    let contexts = roots(&absolute, graph, &mut active)?;
    contexts
        .iter()
        .map(|root| {
            let context = Context {
                document,
                root,
                source_path: &absolute,
                parameters: parameters.clone(),
            };
            render_body(body, &context)
        })
        .collect()
}
