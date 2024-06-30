use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use rusqlite::Connection;

use super::constants::{GITQLITE_DB_NAME, GITQLITE_DIRECTORY_PREFIX};

/**
 * Return a SQLITE connection to the local instance for the repository.
 */
pub fn get_gitqlite_connection() -> crate::Result<Connection> {
    let pwd = std::env::current_dir()?;
    let repo_root = find_gitqlite_root(pwd)?;
    let db_path = repo_root
        .join(GITQLITE_DIRECTORY_PREFIX)
        .join(GITQLITE_DB_NAME);

    let conn = Connection::open(db_path)?;
    Ok(conn)
}

/**
 * Recursively climb the directory to find the root of a gitqlite repository starting from the start directory path
 * by looking for a .gitqlite subdirectory.
 *
 */
pub fn find_gitqlite_root(current_dir: impl AsRef<Path>) -> crate::Result<PathBuf> {
    for dir in current_dir.as_ref().ancestors() {
        if is_gitqlite_root(dir)? {
            return Ok(dir.to_path_buf());
        }
    }

    panic!("fatal: not a git repository (or any of the parent directories): .gitqlite")
}

fn is_gitqlite_root(path: impl AsRef<Path>) -> crate::Result<bool> {
    let path = path.as_ref();
    if !path.is_dir() {
        return Ok(false);
    }

    for entry in fs::read_dir(path).context(format!("Read directory {}", path.display()))? {
        let Ok(entry) = entry else {
            continue;
        };
        if entry.file_name() == ".gitqlite" {
            return Ok(true);
        }
    }

    Ok(false)
}
