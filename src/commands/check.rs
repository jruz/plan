use std::{io, path};

use crate::core::plan;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(" could not read plan: {source}")]
    Read {
        #[source]
        source: io::Error,
    },
    #[error(transparent)]
    Structure(#[from] plan::Error),
}

#[tracing::instrument(skip(file))]
pub async fn run(file: &path::Path) -> Result<(), Error> {
    let source = match tokio::fs::read_to_string(file).await {
        Ok(source) => source,
        Err(source) => return Err(Error::Read { source }),
    };
    plan::check(&source)?;
    Ok(())
}
