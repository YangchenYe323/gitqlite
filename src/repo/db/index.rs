use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::repo::Repository;

use super::{object::FileType, Sha1Id};

/// [`Index`] represents the whole staging area
#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct Index {
    /// File path (relative to repo root) -> Entry
    /// Normally each file has one entry, but when in a merge conflict,
    /// conflicting files will have multiple entries each with a different [`EntryType`]
    entries: BTreeMap<PathBuf, Vec<IndexEntry>>,
}

/// [`MergeStage`] describes the stage of a file during a merge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeStage {
    /// File not in merge conflict
    Normal = 0,
    /// The version of the common ancestor
    CommonAncestor = 1,
    /// Version of our branch (being merged into)
    Ours = 2,
    /// Version of their branch (merge into ours)
    Theirs = 3,
}

/// [`IndexEntry`] represents one entry in the staging area, which is the snapshot of a file
/// in a point in time
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IndexEntry {
    /// The last time the file's metadata has changed, in nanosecond
    pub ctime: i64,
    /// The last time the file's data has changed, in nanosecond
    pub mtime: i64,
    /// The ID of device containing this file
    pub dev: u64,
    /// The inode number of the file
    pub ino: u64,
    /// Type of the entry
    pub type_: FileType,
    /// Permission of the entry
    pub perms: u32,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
    /// Size of the file in bytes
    pub fsize: u64,
    /// SHA of the object
    pub sha: Sha1Id,
    /// If true, we will always assume the file is not changed and will not attempt to do a stat,
    /// e.g., when doing a git status or git diff
    pub flag_assume_valid: bool,
    /// Merge stage of the file
    pub flag_stage: MergeStage,
}

impl Index {
    fn new() -> Index {
        Index {
            entries: BTreeMap::new(),
        }
    }

    /// Create index table and return an empty index
    pub fn create_table(txn: &Transaction) -> crate::Result<Index> {
        let index = Index::new();
        txn.execute("CREATE TABLE Index_ (index_ JSON)", ())?;
        Ok(index)
    }

    /// Read an existing index from the database
    pub fn read_from(txn: &Transaction) -> crate::Result<Index> {
        let s: Option<String> = txn
            .query_row("SELECT index_ FROM Index_", (), |row| row.get(0))
            .optional()?;

        let Some(s) = s else { return Ok(Index::new()) };

        let index =
            serde_json::from_str(&*s).map_err(|e| anyhow!("Invalid index string: {}", s))?;
        Ok(index)
    }

    /// Persist the index to database. Ensure that the table contains a single row
    pub fn persist(&self, txn: &Transaction) -> crate::Result<()> {
        txn.execute("DELETE FROM Index_;", ())?;
        let s = serde_json::to_string(self)?;
        txn.execute("INSERT INTO Index_ (index_) values (?1)", params![s])?;
        Ok(())
    }

    /// Remove the entry for given path from the index.
    /// ! Only removes from index, work tree is not touched and the change is not persisted
    pub fn remove(
        &mut self,
        repo: &Repository,
        path: impl AsRef<Path>,
    ) -> crate::Result<Option<Vec<IndexEntry>>> {
        let name = repo.relative_path(path)?;
        Ok(self.entries.remove(&name))
    }
}
