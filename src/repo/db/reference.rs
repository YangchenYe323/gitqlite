use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::Sha1Id;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ref {
    pub name: String,
    pub commit_id: Sha1Id,
}

impl Ref {
    pub fn create_table(txn: &Transaction) -> crate::Result<()> {
        txn.execute(
            "CREATE TABLE Refs (ref_name TEXT PRIMARY KEY, commit_id BLOB NOT NULL);",
            (),
        )?;
        Ok(())
    }

    pub fn read_from_with_name(txn: &Transaction, name: &str) -> crate::Result<Option<Ref>> {
        txn.query_row_and_then(
            "SELECT ref_name, commit_id FROM Refs WHERE ref_name = ?1",
            [name],
            |row| {
                Ok(Ref {
                    name: row.get(0)?,
                    commit_id: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(anyhow::Error::from)
    }

    pub fn persist(&self, txn: &Transaction) -> crate::Result<()> {
        txn.execute(
            "INSERT OR REPLACE INTO Refs (ref_name, commit_id) VALUES (?1, ?2);",
            params![self.name, self.commit_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_ref_creation_and_persistence() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Ref::create_table(&txn).unwrap();

        let ref_name = "refs/heads/main".to_string();
        let commit_id = Sha1Id([1; 20]);
        let ref_obj = Ref {
            name: ref_name.clone(),
            commit_id,
        };

        ref_obj.persist(&txn).unwrap();

        let retrieved_ref = Ref::read_from_with_name(&txn, &ref_name).unwrap().unwrap();

        assert_eq!(ref_obj.name, retrieved_ref.name);
        assert_eq!(ref_obj.commit_id, retrieved_ref.commit_id);

        txn.commit().unwrap();
    }

    #[test]
    fn test_ref_update() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Ref::create_table(&txn).unwrap();

        let ref_name = "refs/heads/main".to_string();
        let commit_id1 = Sha1Id([1; 20]);
        let ref_obj1 = Ref {
            name: ref_name.clone(),
            commit_id: commit_id1,
        };

        ref_obj1.persist(&txn).unwrap();

        let commit_id2 = Sha1Id([2; 20]);
        let ref_obj2 = Ref {
            name: ref_name.clone(),
            commit_id: commit_id2,
        };

        ref_obj2.persist(&txn).unwrap();

        let retrieved_ref = Ref::read_from_with_name(&txn, &ref_name).unwrap().unwrap();

        assert_eq!(ref_obj2.name, retrieved_ref.name);
        assert_eq!(ref_obj2.commit_id, retrieved_ref.commit_id);

        txn.commit().unwrap();
    }
}
