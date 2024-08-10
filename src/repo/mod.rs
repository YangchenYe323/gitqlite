use std::path::{Path, PathBuf};

use anyhow::anyhow;

pub mod config;
pub mod db;

/// [`Repository`] manages the lifetime of a gitqlite repository
pub struct Repository {
    /// Repo root directory
    root: PathBuf,
}

impl Repository {
    /// Return the path relative to the repo root
    pub fn relative_path(&self, path: impl AsRef<Path>) -> crate::Result<PathBuf> {
        let path = dunce::canonicalize(path)?;
        let relative_path = path.strip_prefix(&self.root).map_err(|_e| {
            anyhow!(
                "Path {} is not within repo {}",
                path.display(),
                self.root.display()
            )
        })?;
        Ok(relative_path.to_path_buf())
    }
}
