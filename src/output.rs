use std::collections::HashMap;

use crate::types::Violation;

pub fn format_violations(violations: &[Violation]) -> String {
    let mut by_file: HashMap<&str, Vec<&Violation>> = HashMap::new();

    for violation in violations {
        by_file.entry(&violation.file).or_default().push(violation);
    }

    let mut files: Vec<_> = by_file.keys().collect();
    files.sort();

    let mut lines: Vec<String> = Vec::new();

    for file in files {
        let Some(file_violations) = by_file.get(file) else {
            continue;
        };

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

pub fn generate_config_toml(violations: &[Violation]) -> String {
    let mut by_rule: HashMap<&str, Vec<&str>> = HashMap::new();

    for violation in violations {
        by_rule
            .entry(&violation.rule)
            .or_default()
            .push(&violation.file);
    }

    let mut lines = vec![
        "[ignore]".to_string(),
        "rules = []".to_string(),
        "files = []".to_string(),
        String::new(),
        "[ignore.by-rule]".to_string(),
    ];

    let mut rules: Vec<_> = by_rule.keys().collect();
    rules.sort();

    for rule in rules {
        let Some(files) = by_rule.get(rule) else {
            continue;
        };
        let mut sorted_files: Vec<_> = files.iter().collect();
        sorted_files.sort();
        sorted_files.dedup();

        let files_str = sorted_files
            .iter()
            .map(|file| format!("\"{file}\""))
            .collect::<Vec<_>>()
            .join(", ");

        lines.push(format!("{rule} = [{files_str}]"));
    }

    lines.push(String::new());
    lines.join("\n")
}
