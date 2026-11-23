use std::{fs, io, path};

use std::io::Write as _;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("plan changed since it was read")]
    Changed,
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("target has no parent directory")]
    Parent,
    #[error(transparent)]
    Persist(#[from] tempfile::PersistError),
}

#[tracing::instrument(skip(file, original, contents))]
pub async fn run(
    file: path::PathBuf,
    original: String,
    contents: String,
) -> Result<path::PathBuf, Error> {
    tokio::task::spawn_blocking(move || replace(&file, &original, &contents)).await?
}

fn replace(file: &path::Path, original: &str, contents: &str) -> Result<path::PathBuf, Error> {
    let directory = file.parent().ok_or(Error::Parent)?;
    let permissions = fs::metadata(file)?.permissions();
    let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
    let bytes = contents.as_bytes();
    temporary.write_all(bytes)?;
    let handle = temporary.as_file();
    handle.set_permissions(permissions)?;
    handle.sync_all()?;
    let current = fs::read_to_string(file)?;
    if current != original {
        return Err(Error::Changed);
    }
    let backup = snapshot(directory, original)?;
    temporary.persist(file)?;
    Ok(backup)
}

fn snapshot(directory: &path::Path, original: &str) -> Result<path::PathBuf, Error> {
    let backups = directory.join(".plan");
    fs::create_dir_all(&backups)?;
    ensure_ignored(&backups)?;

    let now = jiff::Timestamp::now();
    let timestamp = now.strftime("%Y-%m-%dT%H-%M-%SZ");
    let prefix = format!("PLAN-{timestamp}");
    let mut snapshot = tempfile::Builder::new()
        .prefix(&prefix)
        .suffix(".md")
        .tempfile_in(&backups)?;
    let bytes = original.as_bytes();
    snapshot.write_all(bytes)?;
    snapshot.as_file().sync_all()?;
    let name = format!("{prefix}.md");
    let path = backups.join(name);
    match snapshot.persist_noclobber(&path) {
        Ok(_) => Ok(path),
        Err(error) if error.error.kind() == io::ErrorKind::AlreadyExists => {
            let (_, unique_path) = error.file.keep()?;
            Ok(unique_path)
        }
        Err(error) => {
            let failure = Error::Persist(error);
            Err(failure)
        }
    }
}

fn ensure_ignored(directory: &path::Path) -> Result<(), Error> {
    let path = directory.join(".gitignore");
    let mut ignore = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    let mut rules = String::new();
    io::Read::read_to_string(&mut ignore, &mut rules)?;
    if rules.lines().last() != Some("*") {
        if !rules.is_empty() && !rules.ends_with('\n') {
            ignore.write_all(b"\n")?;
        }
        ignore.write_all(b"*\n")?;
        ignore.sync_all()?;
    }
    Ok(())
}
