use std::fs;
use std::path::Path;

use color_eyre::Result;

use crate::types::{Config, Violation};

const CONFIG_FILE: &str = "slop-detector.toml";

pub fn load_config() -> Result<Config> {
    let path = Path::new(CONFIG_FILE);
    if !path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

fn should_ignore_file(file: &str, config: &Config) -> bool {
    let Some(ignore) = &config.ignore else {
        return false;
    };

    let Some(files) = &ignore.files else {
        return false;
    };

    files.iter().any(|pattern| file.contains(pattern))
}

fn should_ignore_rule(file: &str, rule: &str, config: &Config) -> bool {
    let Some(ignore) = &config.ignore else {
        return false;
    };

    if let Some(rules) = &ignore.rules
        && rules.contains(&rule.to_string())
    {
        return true;
    }

    if let Some(by_rule) = &ignore.by_rule
        && let Some(patterns) = by_rule.get(rule)
        && patterns.iter().any(|pattern| file.contains(pattern))
    {
        return true;
    }

    false
}

pub fn filter_violations(violations: Vec<Violation>, config: &Config) -> Vec<Violation> {
    violations
        .into_iter()
        .filter(|violation| {
            !should_ignore_file(&violation.file, config)
                && !should_ignore_rule(&violation.file, &violation.rule, config)
        })
        .collect()
}
