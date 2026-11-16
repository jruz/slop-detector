use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Violation {
    pub errors: Vec<String>,
    pub file: String,
    pub hint: Option<String>,
    pub rule: String,
}

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipCategory {
    ParseError,
    UnsupportedStaticAnalysis,
}

#[derive(Debug, Clone)]
pub struct SkippedAnalysis {
    pub category: SkipCategory,
    pub language: &'static str,
    pub recipe: Option<String>,
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub errors: Vec<AppError>,
    pub file_count: usize,
    pub violations: Vec<Violation>,
    pub skipped: Vec<SkippedAnalysis>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("Failed to glob {pattern}: {message}")]
    Glob { pattern: String, message: String },

    #[error("Failed to parse {file}: {message}")]
    Parse { file: String, message: String },

    #[error("Failed to read {file}: {message}")]
    Read { file: String, message: String },

    #[error("Could not analyze recipe {recipe} from {file}: {message}")]
    Recipe {
        file: String,
        recipe: String,
        message: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    pub ignore: Option<IgnoreConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct IgnoreConfig {
    #[serde(rename = "by-rule")]
    pub by_rule: Option<HashMap<String, Vec<String>>>,
    pub files: Option<Vec<String>>,
    pub rules: Option<Vec<String>>,
}
