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
pub enum AnalysisCategory {
    ParseError,
    UnsupportedStaticAnalysis,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisFailure {
    pub category: AnalysisCategory,
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
    pub skipped: Vec<AnalysisFailure>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum AppError {
    #[error("Failed to glob {pattern}: {message}")]
    Glob { pattern: String, message: String },

    #[error("Failed to parse {file}: {message}")]
    Parse { file: String, message: String },

    #[error("Failed to read {file}: {message} ({kind})")]
    Read {
        file: String,
        message: String,
        kind: std::io::ErrorKind,
    },

    #[error("Could not analyze recipe {recipe} from {file}: {message}")]
    Recipe {
        file: String,
        recipe: String,
        message: String,
    },
}

pub fn read_error(file: String, error: &std::io::Error) -> AppError {
    AppError::Read {
        file,
        message: error.to_string(),
        kind: error.kind(),
    }
}

pub fn partition_analysis_errors(
    errors: Vec<AppError>,
    language: &'static str,
    recipe_category: AnalysisCategory,
) -> (Vec<AppError>, Vec<AnalysisFailure>) {
    let mut cli_errors = Vec::new();
    let mut failures = Vec::new();
    for error in errors {
        match error {
            AppError::Parse { file, message } => failures.push(AnalysisFailure {
                category: AnalysisCategory::ParseError,
                language,
                file: file.trim_start_matches("./").to_owned(),
                recipe: None,
                reason: message,
            }),
            AppError::Recipe {
                file,
                recipe,
                message,
            } => failures.push(AnalysisFailure {
                category: recipe_category,
                language,
                file: file.trim_start_matches("./").to_owned(),
                recipe: Some(recipe),
                reason: message,
            }),
            AppError::Glob { .. } | AppError::Read { .. } => cli_errors.push(error),
        }
    }
    (cli_errors, failures)
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
