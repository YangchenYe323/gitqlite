use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha1::Digest;

use super::{hash::Hashable, object::Object, IdType, NoId, Sha1Id};

/// [`Blob`] represents a snapshot of a file in the gitqlite repository
#[derive(Debug, Serialize, Deserialize)]
pub struct Blob<ID: IdType<ID>> {
    pub blob_id: ID,
    pub data: Vec<u8>,
}

impl Hashable for Blob<NoId> {
    fn hash(&self, mut sha: sha1::Sha1) -> super::Sha1Id {
        sha.update(&self.data);
        Sha1Id(sha.finalize().into())
    }
}

impl Blob<NoId> {
    pub fn new(data: Vec<u8>) -> Blob<NoId> {
        Self {
            blob_id: NoId,
            data,
        }
    }

    pub fn with_id(self) -> Blob<Sha1Id> {
        let id = self.hash(sha1::Sha1::new());
        Blob {
            blob_id: id,
            data: self.data,
        }
    }
}

impl Object for Blob<Sha1Id> {
    type Id = Sha1Id;

    fn type_(&self) -> super::object::ObjectType {
        super::object::ObjectType::Blob
    }

    fn create_table(txn: &rusqlite::Transaction) -> crate::Result<()> {
        txn.execute(
            "CREATE TABLE Blobs (blob_id TEXT PRIMARY KEY, data BLOB NOT NULL);",
            (),
        )?;
        Ok(())
    }

    fn read_by_id(txn: &rusqlite::Transaction, id: Self::Id) -> crate::Result<Option<Self>> {
        txn.query_row_and_then(
            "SELECT blob_id, data FROM Blobs WHERE blob_id = ?1;",
            [id],
            |row| {
                let blob_id = row.get(0)?;
                let data = row.get(1)?;
                Ok(Blob { blob_id, data })
            },
        )
        .optional()
        .map_err(anyhow::Error::from)
    }

    fn persist(&self, txn: &rusqlite::Transaction) -> crate::Result<()> {
        txn.execute(
            "INSERT OR IGNORE INTO Blobs (blob_id, data) VALUES (?1, ?2);",
            params![&self.blob_id, &self.data],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_blob_creation_and_persistence() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Blob::<Sha1Id>::create_table(&txn).unwrap();

        let data = vec![1, 2, 3, 4, 5];
        let blob = Blob::new(data.clone()).with_id();

        blob.persist(&txn).unwrap();

        let retrieved_blob = Blob::read_by_id(&txn, blob.blob_id).unwrap().unwrap();

        assert_eq!(blob.blob_id, retrieved_blob.blob_id);
        assert_eq!(blob.data, retrieved_blob.data);

        txn.commit().unwrap();
    }

    #[test]
    fn test_blob_hashing() {
        let data1 = vec![1, 2, 3, 4, 5];
        let data2 = vec![1, 2, 3, 4, 5];
        let data3 = vec![5, 4, 3, 2, 1];

        let blob1 = Blob::new(data1);
        let blob2 = Blob::new(data2);
        let blob3 = Blob::new(data3);

        assert_eq!(blob1.hash(sha1::Sha1::new()), blob2.hash(sha1::Sha1::new()));
        assert_ne!(blob1.hash(sha1::Sha1::new()), blob3.hash(sha1::Sha1::new()));
    }
}
