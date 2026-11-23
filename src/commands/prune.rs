use std::{io, path};

use crate::{core::plan, utils::replace_file};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(" could not read plan: {source}")]
    Read {
        #[from]
        source: io::Error,
    },
    #[error(transparent)]
    Structure(#[from] plan::Error),
    #[error(" could not replace plan: {source}")]
    Write {
        #[from]
        source: replace_file::Error,
    },
}

pub enum Mode {
    Apply,
    Preview,
}

pub enum Outcome {
    NoChange,
    Preview(String),
    Saved(path::PathBuf),
}

#[tracing::instrument(skip(file, mode))]
pub async fn run(file: &path::Path, mode: Mode) -> Result<Outcome, Error> {
    let target = tokio::fs::canonicalize(file).await?;
    let source = tokio::fs::read_to_string(&target).await?;
    let Some(updated) = plan::prune(&source)? else {
        return Ok(Outcome::NoChange);
    };
    let outcome = match mode {
        Mode::Apply => {
            let backup = replace_file::run(target, source, updated).await?;
            Outcome::Saved(backup)
        }
        Mode::Preview => Outcome::Preview(updated),
    };
    Ok(outcome)
}
