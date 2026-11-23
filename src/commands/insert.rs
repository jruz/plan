use std::{io, num, path};

use crate::{
    core::{insert, plan},
    utils::replace_file,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(" supply either a title or `--from <FILE>`")]
    Input,
    #[error(" could not read phase file {file:?}: {source}")]
    ReadFragment {
        file: path::PathBuf,
        #[source]
        source: io::Error,
    },
    #[error(" could not read plan: {source}")]
    ReadPlan {
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

#[tracing::instrument(skip(file, title, from, mode))]
pub async fn run(
    file: &path::Path,
    position: num::NonZeroUsize,
    title: Option<&str>,
    from: Option<&path::Path>,
    mode: replace_file::Mode,
) -> Result<replace_file::Outcome, Error> {
    let target = tokio::fs::canonicalize(file).await?;
    let source = tokio::fs::read_to_string(&target).await?;
    let updated = match (title, from) {
        (Some(title), None) => {
            let input = insert::Input::Title(title);
            insert::insert(&source, position, input)?
        }
        (None, Some(fragment_file)) => {
            let fragment = read_fragment(fragment_file).await?;
            let input = insert::Input::Fragment(&fragment);
            insert::insert(&source, position, input)?
        }
        (None, None) | (Some(_), Some(_)) => return Err(Error::Input),
    };
    let outcome = replace_file::finish(target, source, updated, mode).await?;
    Ok(outcome)
}

async fn read_fragment(file: &path::Path) -> Result<String, Error> {
    match tokio::fs::read_to_string(file).await {
        Ok(fragment) => Ok(fragment),
        Err(source) => Err(Error::ReadFragment {
            file: file.to_path_buf(),
            source,
        }),
    }
}
