use std::collections::{BTreeMap, BTreeSet};

use crate::types::{Config, Violation};

pub fn format_violations(violations: &[Violation]) -> String {
    let mut by_file: BTreeMap<&str, Vec<&Violation>> = BTreeMap::new();

    for violation in violations {
        by_file.entry(&violation.file).or_default().push(violation);
    }

    let mut lines: Vec<String> = Vec::new();

    for (file, file_violations) in &by_file {
        lines.push(format!("\n{file}"));
        for violation in file_violations {
            lines.push(format!("  \u{2717} {}", violation.rule));
            for error in &violation.errors {
                lines.push(format!("    - {error}"));
            }
            if let Some(hint) = &violation.hint {
                lines.push(format!("    hint: {hint}"));
            }
        }
    }

    lines.join("\n")
}

fn merge_by_rule(
    existing: &Config,
    new_violations: &BTreeMap<&str, Vec<&str>>,
) -> BTreeMap<String, Vec<String>> {
    let mut merged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    if let Some(ignore) = &existing.ignore
        && let Some(by_rule) = &ignore.by_rule
    {
        let mut sorted_rules: Vec<_> = by_rule.keys().collect();
        sorted_rules.sort();
        for rule in sorted_rules {
            if let Some(files) = by_rule.get(rule) {
                let rule_name = rule.clone();
                let files_to_merge = files.clone();
                merged.entry(rule_name).or_default().extend(files_to_merge);
            }
        }
    }

    for (rule, files) in new_violations {
        let rule_name = rule.to_string();
        let entry = merged.entry(rule_name).or_default();
        for file in files {
            let file_name = file.to_string();
            entry.insert(file_name);
        }
    }

    merged
        .into_iter()
        .map(|(rule, files)| (rule, files.into_iter().collect()))
        .collect()
}

fn merge_files(existing: &Config) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();

    if let Some(ignore) = &existing.ignore
        && let Some(existing_files) = &ignore.files
    {
        let existing_files = existing_files.clone();
        files.extend(existing_files);
    }

    files.sort();
    files.dedup();
    files
}

fn merge_rules(existing: &Config) -> Vec<String> {
    let mut rules: Vec<String> = Vec::new();

    if let Some(ignore) = &existing.ignore
        && let Some(existing_rules) = &ignore.rules
    {
        let existing_rules = existing_rules.clone();
        rules.extend(existing_rules);
    }

    rules.sort();
    rules.dedup();
    rules
}

fn format_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }

    let mut lines = vec!["[".to_string()];
    for item in items {
        lines.push(format!("    \"{item}\","));
    }
    let closing_bracket = "]".to_string();
    lines.push(closing_bracket);
    lines.join("\n")
}

pub fn generate_config_toml(violations: &[Violation], existing: &Config) -> String {
    let mut by_rule: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for violation in violations {
        by_rule
            .entry(&violation.rule)
            .or_default()
            .push(&violation.file);
    }

    let rules = merge_rules(existing);
    let files = merge_files(existing);
    let merged_by_rule = merge_by_rule(existing, &by_rule);

    let mut lines = vec![
        "[ignore]".to_string(),
        format!("rules = {}", format_string_array(&rules)),
        format!("files = {}", format_string_array(&files)),
        String::new(),
        "[ignore.by-rule]".to_string(),
    ];

    for (rule, rule_files) in &merged_by_rule {
        lines.push(format!("{rule} = {}", format_string_array(rule_files)));
    }

    let blank_line = String::new();
    lines.push(blank_line);
    lines.join("\n")
}
