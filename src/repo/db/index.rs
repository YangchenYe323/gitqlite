use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::anyhow;
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::{
    object::{FileType, Object, ObjectType},
    Sha1Id,
};

/// [`Index`] represents the whole staging area
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

    /// Remove the entry for given path from the index.
    /// ! Only removes from index, work tree is not touched and the change is not persisted
    pub fn remove(
        &mut self,
        repo_root: impl AsRef<Path>,
        path: impl AsRef<Path>,
    ) -> crate::Result<Option<Vec<IndexEntry>>> {
        let path = dunce::canonicalize(path.as_ref())?;
        let name = path.strip_prefix(repo_root.as_ref()).map_err(|_e| {
            anyhow!(
                "Path {} is not inside repository {}",
                path.display(),
                repo_root.as_ref().display()
            )
        })?;
        Ok(self.entries.remove(name))
    }
}
impl Object for Index {
    type Id = ();
    /// Create index table and return an empty index
    fn create_table(txn: &Transaction) -> crate::Result<()> {
        txn.execute("CREATE TABLE Index_ (index_ JSON);", ())?;
        Ok(())
    }

    /// Read an existing index from the database
    fn read_by_id(txn: &Transaction, _id: Self::Id) -> crate::Result<Option<Index>> {
        let s: Option<String> = txn
            .query_row("SELECT index_ FROM Index_;", (), |row| row.get(0))
            .optional()?;

        let Some(s) = s else { return Ok(None) };

        let index =
            serde_json::from_str(&*s).map_err(|_e| anyhow!("Invalid index string: {}", s))?;
        Ok(Some(index))
    }

    /// Persist the index to database. Ensure that the table contains a single row
    fn persist(&self, txn: &Transaction) -> crate::Result<()> {
        txn.execute("DELETE FROM Index_;", ())?;
        let s = serde_json::to_string(self)?;
        txn.execute("INSERT INTO Index_ (index_) values (?1);", params![s])?;
        Ok(())
    }

    fn type_(&self) -> ObjectType {
        ObjectType::Index
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    #[test]
    fn test_index_creation_and_persistence() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Index::create_table(&txn).unwrap();

        let mut index = Index::new();
        let entry = IndexEntry {
            ctime: 1000,
            mtime: 2000,
            dev: 1,
            ino: 2,
            type_: FileType::Regular,
            perms: 0o644,
            uid: 1000,
            gid: 1000,
            fsize: 100,
            sha: Sha1Id([1; 20]),
            flag_assume_valid: false,
            flag_stage: MergeStage::Normal,
        };
        index
            .entries
            .insert(PathBuf::from("file1.txt"), vec![entry]);

        index.persist(&txn).unwrap();

        let retrieved_index = Index::read_by_id(&txn, ()).unwrap().unwrap();

        assert_eq!(index, retrieved_index);

        txn.commit().unwrap();
    }

    #[test]
    fn test_index_remove() {
        const FILE_NAME: &str = "file.txt";
        let dir = tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        let _f = fs::File::create(&path).unwrap();

        let mut index = Index::new();
        let entry = IndexEntry {
            ctime: 1000,
            mtime: 2000,
            dev: 1,
            ino: 2,
            type_: FileType::Regular,
            perms: 0o644,
            uid: 1000,
            gid: 1000,
            fsize: 100,
            sha: Sha1Id([1; 20]),
            flag_assume_valid: false,
            flag_stage: MergeStage::Normal,
        };
        index
            .entries
            .insert(PathBuf::from(FILE_NAME), vec![entry.clone()]);

        let removed = index.remove(dir.path(), &path).unwrap();

        assert_eq!(removed, Some(vec![entry]));
        assert!(index.entries.is_empty());
    }
}
