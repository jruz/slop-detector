use std::{env, fs, io::Write as _, path};

use crate::types;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("home directory is unavailable")]
    MissingHome,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] time::error::Format),
}

#[derive(serde::Serialize)]
struct Diagnostic<'diagnostic> {
    timestamp: &'diagnostic str,
    version: &'static str,
    revision: &'static str,
    project: &'diagnostic path::Path,
    language: &'diagnostic str,
    file: &'diagnostic path::Path,
    recipe: &'diagnostic Option<String>,
    category: types::SkipCategory,
    reason: &'diagnostic str,
}

pub fn append(skipped: &[types::SkippedAnalysis]) -> Result<(), Error> {
    if !skipped.is_empty() {
        let home = env::home_dir().ok_or(Error::MissingHome)?;
        let directory = home.join(".local/state/slop-detector");
        fs::create_dir_all(&directory)?;
        let log_path = directory.join("skipped.jsonl");
        let mut log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        let project = env::current_dir()?;
        let timestamp = time::OffsetDateTime::now_utc();
        let timestamp = timestamp.format(&time::format_description::well_known::Rfc3339)?;
        for analysis in skipped {
            let file = path::Path::new(&analysis.file);
            let file: path::PathBuf = file
                .components()
                .filter(|component| *component != path::Component::CurDir)
                .collect();
            let diagnostic = Diagnostic {
                timestamp: &timestamp,
                version: env!("CARGO_PKG_VERSION"),
                revision: env!("REVISION"),
                project: &project,
                language: analysis.language,
                file: &file,
                recipe: &analysis.recipe,
                category: analysis.category,
                reason: &analysis.reason,
            };
            let mut record = serde_json::to_vec(&diagnostic)?;
            record.push(b'\n');
            log.write_all(&record)?;
        }
    }
    Ok(())
}
