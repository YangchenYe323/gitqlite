use rusqlite::Transaction;
use serde::{Deserialize, Serialize};

/// [`FileType`] represents a file type on the file system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Directory,
    Regular,
    Symlink,
}

impl FileType {
    pub fn num(self) -> u32 {
        match self {
            FileType::Directory => 0o040,
            FileType::Regular => 0o100,
            FileType::Symlink => 0o120,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    Index,
    Head,
    Commit,
    Tree,
    Blob,
}

/// [`Object`] represents a generic object in gitqlite database
pub trait Object: Sized {
    type Id;

    fn type_(&self) -> ObjectType;

    /// Initialize table in database
    fn create_table(txn: &Transaction) -> crate::Result<()>;

    /// Read one instance by Id
    fn read_by_id(txn: &Transaction, id: Self::Id) -> crate::Result<Option<Self>>;

    /// Persist on instance
    fn persist(&self, txn: &Transaction) -> crate::Result<()>;
}
