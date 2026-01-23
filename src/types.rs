use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Violation {
    pub errors: Vec<String>,
    pub file: String,
    pub hint: Option<String>,
    pub rule: String,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub errors: Vec<AppError>,
    pub file_count: usize,
    pub violations: Vec<Violation>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("Failed to parse {file}: {message}")]
    ParseFailed { file: String, message: String },

    #[error("Failed to read {file}")]
    ReadFailed { file: String, message: String },
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub ignore: Option<IgnoreConfig>,
}

#[derive(Debug, Default, Deserialize)]
pub struct IgnoreConfig {
    #[serde(rename = "by-rule")]
    pub by_rule: Option<HashMap<String, Vec<String>>>,
    pub files: Option<Vec<String>>,
    pub rules: Option<Vec<String>>,
}
