use std::{collections::BTreeMap, path::PathBuf};

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha1::Digest as _;

use super::{
    hash::Hashable,
    object::{FileType, Object, ObjectType},
    IdType, NoId, Sha1Id,
};

/// [`Tree`] represents a snapshot of a directory in gitqlite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tree<ID: IdType<ID>> {
    pub tree_id: ID,
    /// Map from the path segment -> tree entry
    /// ## Example
    /// If a directory is of the following structure:
    /// /a
    ///   /b
    ///     /1.txt
    /// Then the tree representing the directory would be like:
    ///
    /// - a's tree entries:
    /// b -> [an entry for b]
    ///
    /// - b's tree entries:
    /// 1.txt -> [an entry for 1.txt]
    pub entries: BTreeMap<PathBuf, TreeEntry>,
}

/// [`TreeEntry`] represents one entry in a gitqlite tree, which is essentially
/// a pointer to either a [`Blob`] or another [`Tree`]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeEntry {
    /// Type of object pointed to by this entry.
    /// Only valid values are Blob/Tree
    pub object_type: ObjectType,
    /// Type of the underlying file in the file system.
    pub file_type: FileType,
    /// Id of the object pointed by this entry.
    pub id: Sha1Id,
    /// Permission of the file pointed by this entry. If the entry
    /// points to a tree/directory, permission is always 000
    pub perms: u32,
}

impl Hashable for Tree<NoId> {
    fn hash(&self, mut sha: sha1::Sha1) -> Sha1Id {
        let s = serde_json::to_vec(self).expect("Serialize tree failed");
        sha.update(s);
        Sha1Id(sha.finalize().into())
    }
}

impl Tree<NoId> {
    pub fn with_id(self) -> Tree<Sha1Id> {
        let id = self.hash(sha1::Sha1::new());
        Tree {
            tree_id: id,
            entries: self.entries,
        }
    }
}

impl Object for Tree<Sha1Id> {
    type Id = Sha1Id;

    fn type_(&self) -> ObjectType {
        ObjectType::Tree
    }

    fn create_table(txn: &rusqlite::Transaction) -> crate::Result<()> {
        txn.execute(
            "CREATE TABLE Trees (tree_id BLOB PRIMARY KEY, data JSON NOT NULL);",
            (),
        )?;
        Ok(())
    }

    fn read_by_id(txn: &rusqlite::Transaction, id: Self::Id) -> crate::Result<Option<Self>> {
        txn.query_row_and_then(
            "SELECT tree_id, data FROM Trees WHERE tree_id = (?1);",
            [id],
            |row| {
                let tree_id = row.get(0)?;
                let data: String = row.get(1)?;
                let entries = serde_json::from_str(&data)
                    .expect("Failed to deserialize tree (is database corrupted?)");
                Ok(Tree { tree_id, entries })
            },
        )
        .optional()
        .map_err(anyhow::Error::from)
    }

    fn persist(&self, txn: &rusqlite::Transaction) -> crate::Result<()> {
        let id = self.tree_id;
        let s = serde_json::to_string(&self.entries).expect("Failed to serialize tree");
        txn.execute(
            "INSERT OR IGNORE INTO Trees (tree_id, data) VALUES (?1, ?2);",
            params![id, s],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_tree_creation_and_persistence() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Tree::<Sha1Id>::create_table(&txn).unwrap();

        let mut entries = BTreeMap::new();
        entries.insert(
            PathBuf::from("file1.txt"),
            TreeEntry {
                object_type: ObjectType::Blob,
                file_type: FileType::Regular,
                id: Sha1Id([1; 20]),
                perms: 0o644,
            },
        );

        let tree = Tree {
            tree_id: NoId,
            entries,
        }
        .with_id();

        tree.persist(&txn).unwrap();

        let retrieved_tree = Tree::read_by_id(&txn, tree.tree_id).unwrap().unwrap();

        assert_eq!(tree.tree_id, retrieved_tree.tree_id);
        assert_eq!(tree.entries, retrieved_tree.entries);

        txn.commit().unwrap();
    }

    #[test]
    fn test_tree_hashing() {
        let mut entries1 = BTreeMap::new();
        entries1.insert(
            PathBuf::from("file1.txt"),
            TreeEntry {
                object_type: ObjectType::Blob,
                file_type: FileType::Regular,
                id: Sha1Id([1; 20]),
                perms: 0o644,
            },
        );

        let mut entries2 = entries1.clone();

        let mut entries3 = BTreeMap::new();
        entries3.insert(
            PathBuf::from("file2.txt"),
            TreeEntry {
                object_type: ObjectType::Blob,
                file_type: FileType::Regular,
                id: Sha1Id([2; 20]),
                perms: 0o644,
            },
        );

        let tree1 = Tree {
            tree_id: NoId,
            entries: entries1,
        };
        let tree2 = Tree {
            tree_id: NoId,
            entries: entries2,
        };
        let tree3 = Tree {
            tree_id: NoId,
            entries: entries3,
        };

        assert_eq!(tree1.hash(sha1::Sha1::new()), tree2.hash(sha1::Sha1::new()));
        assert_ne!(tree1.hash(sha1::Sha1::new()), tree3.hash(sha1::Sha1::new()));
    }
}
