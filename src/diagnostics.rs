use std::{env, fs, io::Write as _, path};

use crate::types;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("neither XDG_STATE_HOME nor HOME specifies an absolute state directory")]
    MissingStateDirectory,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Time(#[from] time::error::Format),
}

#[derive(serde::Serialize)]
#[serde(tag = "event", rename_all = "kebab-case")]
pub enum Event<'event> {
    AnalysisFailure(&'event types::AnalysisFailure),
    OperationFailure {
        operation: &'static str,
        path: Option<String>,
        cause: String,
    },
}

#[derive(serde::Serialize)]
struct Record<'record> {
    timestamp: &'record str,
    level: &'static str,
    application: &'static str,
    version: &'static str,
    revision: &'static str,
    project: &'record path::Path,
    #[serde(flatten)]
    event: &'record Event<'record>,
}

fn state_directory() -> Result<path::PathBuf, Error> {
    if let Some(state) = env::var_os("XDG_STATE_HOME").map(path::PathBuf::from)
        && state.is_absolute()
    {
        return Ok(state);
    }
    if let Some(home) = env::var_os("HOME").map(path::PathBuf::from)
        && home.is_absolute()
    {
        let state = home.join(".local/state");
        return Ok(state);
    }
    Err(Error::MissingStateDirectory)
}

pub fn append(events: &[Event<'_>]) -> Result<(), Error> {
    if events.is_empty() {
        Ok(())
    } else {
        append_records(events)
    }
}

fn append_records(events: &[Event<'_>]) -> Result<(), Error> {
    let application = env!("CARGO_PKG_NAME");
    let state = state_directory()?;
    let directory = state.join(application);
    let project = env::current_dir()?;
    let now = time::OffsetDateTime::now_utc();
    let timestamp = now.format(&time::format_description::well_known::Rfc3339)?;
    fs::create_dir_all(&directory)?;
    let log_path = directory.join("errors.jsonl");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    for event in events {
        let record = Record {
            timestamp: &timestamp,
            level: "ERROR",
            application,
            version: env!("CARGO_PKG_VERSION"),
            revision: env!("REVISION"),
            project: &project,
            event,
        };
        let mut line = serde_json::to_vec(&record)?;
        line.push(b'\n');
        log.write_all(&line)?;
    }
    Ok(())
}
