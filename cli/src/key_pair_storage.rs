// Radicle Registry
// Copyright (C) 2019 Monadic GmbH <radicle@monadic.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Manages key pairs stored in the filesystem,
//! providing ways to store and retrieve them.

use directories::BaseDirs;
use sp_core::serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use thiserror::Error as ThisError;

use std::io::Error as IOError;
use std::path::PathBuf;

/// The data that is stored in the filesystem relative
/// to a key pair. The name of the key pair is used as
/// the key to this value, therefore not included here.
#[derive(Serialize, Deserialize, Clone)]
pub struct KeyPairData {
    pub seed: Seed,
}

/// The seed from which a key pair
/// can be deterministically generated.
type Seed = [u8; 32];

#[derive(Debug, ThisError)]
pub enum Error {
    /// A key pair with the given name already exists
    #[error("A key pair with the given name already exists")]
    AlreadyExists(),

    /// Failed to write to the key-pairs file
    #[error("{}", io_error_message("write"))]
    FailedWrite(#[from] WritingError),

    /// Failed to read the key-pairs file
    #[error("{}", io_error_message("read"))]
    FailedRead(#[from] ReadingError),

    /// Cannot read directory
    #[error("Cannot read directory '{1}'")]
    CannotReadDirectory(#[source] IOError, PathBuf),

    /// Cannot create directory
    #[error("Cannot create directory '{1}'")]
    CannotCreateDirectory(#[source] IOError, PathBuf),

    /// Could not find a key pair with the given name
    #[error("Could not find a key pair with the given name")]
    NotFound(),
}

fn io_error_message(action: &str) -> String {
    let path = build_path(FILE);
    format!(
        "Failed to {} the key-pairs file: '{}'",
        action,
        path.display()
    )
}

/// Possible errors when writing to the key-pairs file.
#[derive(Debug, ThisError)]
pub enum WritingError {
    #[error(transparent)]
    IO(IOError),

    #[error(transparent)]
    Serialization(serde_json::Error),
}

/// Possible errors when reading the key-pairs file.
#[derive(Debug, ThisError)]
pub enum ReadingError {
    #[error(transparent)]
    IO(IOError),

    #[error(transparent)]
    Deserialization(serde_json::Error),
}

/// Add a key pair to the storage.
///
/// Fails if a key pair with the given `name` already exists.
/// It can also fail from IO and Serde Json errors.
pub fn add(name: String, data: KeyPairData) -> Result<(), Error> {
    let mut key_pairs = list()?;
    if key_pairs.contains_key(&name) {
        return Err(Error::AlreadyExists());
    }

    key_pairs.insert(name, data);
    update(key_pairs)
}

/// List all the stored key-pairs.
///
/// It can fail from IO and Serde Json errors.
pub fn list() -> Result<HashMap<String, KeyPairData>, Error> {
    let path_buf = get_or_create_path()?;
    let file = File::open(path_buf.as_path()).map_err(ReadingError::IO)?;
    let key_pairs: HashMap<String, KeyPairData> =
        serde_json::from_reader(&file).map_err(ReadingError::Deserialization)?;
    Ok(key_pairs)
}

/// Get a key pair by name.
///
/// It can fail from IO and Serde Json errors, or if no such
/// key pair is found.
pub fn get(name: &str) -> Result<KeyPairData, Error> {
    list()?.get(name).map(Clone::clone).ok_or(Error::NotFound())
}

fn update(key_pairs: HashMap<String, KeyPairData>) -> Result<(), Error> {
    let path_buf = get_or_create_path()?;
    let new_content =
        serde_json::to_string_pretty(&key_pairs).map_err(WritingError::Serialization)?;
    std::fs::write(path_buf.as_path(), new_content.as_bytes()).map_err(WritingError::IO)?;
    Ok(())
}

/// The file where the user key-pairs are stored.
const FILE: &str = "key-pairs.json";

/// Get the path to the key-pairs file on disk.
///
/// If the file does not yet exist, create it and initialize
/// it with an empty object so that it can be deserialized
/// as an empty HashMap<String, KeyPairData>.
fn get_or_create_path() -> Result<PathBuf, Error> {
    let path_buf = build_path(FILE);
    let path = path_buf.as_path();
    dir_ready(path.parent().unwrap().to_path_buf())?;

    let old_path = build_path("accounts.json");
    if !path.exists() && old_path.exists() {
        println!("=> Migrating the key-pair storage to the latest version...");
        std::fs::rename(old_path, path).map_err(WritingError::IO)?;
        println!("✓ Done")
    }

    if !path.exists() {
        std::fs::write(path, b"{}").map_err(WritingError::IO)?;
    }

    Ok(path_buf)
}

/// Ensure that the given directory path is ready to be used.
/// Fails with
///   * [Error::CannotCreateDirectory] if the directory
///     does not exist and fails to be created.
///   * [Error::CannotReadDirectory] if the directory
///     does exist but can not be read.
fn dir_ready(dir: PathBuf) -> Result<PathBuf, Error> {
    std::fs::create_dir_all(&dir).map_err(|err| Error::CannotCreateDirectory(err, dir.clone()))?;
    File::open(&dir).map_err(|err| Error::CannotReadDirectory(err, dir.clone()))?;
    Ok(dir)
}

/// Build the path to the given filename under [dir()].
fn build_path(filename: &str) -> PathBuf {
    dir().join(filename)
}

fn dir() -> PathBuf {
    BaseDirs::new()
        .unwrap()
        .data_dir()
        .join("radicle-registry-cli")
}
