//! Bounded JSON reads and atomic writes in application-owned directories.

use crate::error::ClientError;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

const MAX_BYTES: u64 = 1024 * 1024;

/// The content of a JSON file, or nothing when the file is not there yet.
pub fn read<T: DeserializeOwned>(path: &Path) -> Result<Option<T>, ClientError> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ClientError::StorageUnavailable),
    };
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ClientError::StorageUnavailable)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ClientError::StorageUnavailable);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| ClientError::StorageUnavailable)
}

/// Writes a JSON file to a temporary file beside it, then puts it in place.
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), ClientError> {
    let parent = path.parent().ok_or(ClientError::StorageUnavailable)?;
    fs::create_dir_all(parent).map_err(|_| ClientError::StorageUnavailable)?;
    let bytes = serde_json::to_vec(value).map_err(|_| ClientError::StorageUnavailable)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| ClientError::StorageUnavailable)?;
    temporary
        .write_all(&bytes)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|_| ClientError::StorageUnavailable)?;
    temporary
        .persist(path)
        .map_err(|_| ClientError::StorageUnavailable)?;
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| ClientError::StorageUnavailable)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/storage/mod.rs"]
mod tests;
