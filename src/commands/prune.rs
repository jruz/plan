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

#[tracing::instrument(skip(file, mode))]
pub async fn run(
    file: &path::Path,
    mode: replace_file::Mode,
) -> Result<replace_file::Outcome, Error> {
    let target = tokio::fs::canonicalize(file).await?;
    let source = tokio::fs::read_to_string(&target).await?;
    let Some(updated) = plan::prune(&source)? else {
        return Ok(replace_file::Outcome::NoChange);
    };
    let outcome = replace_file::finish(target, source, updated, mode).await?;
    Ok(outcome)
}
