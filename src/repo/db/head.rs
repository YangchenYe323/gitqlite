use anyhow::anyhow;
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

use super::{object::Object, Sha1Id};

// [`Head`] represents the current HEAD of the repository.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Head {
    /// Branch points to a reference
    Branch(String),
    /// Detached head points directly to a commit
    Commit(Sha1Id),
}

impl Object for Head {
    type Id = ();

    fn type_(&self) -> super::object::ObjectType {
        super::object::ObjectType::Head
    }

    fn create_table(txn: &Transaction) -> crate::Result<()> {
        txn.execute("CREATE TABLE Head (head JSON);", ())?;
        Ok(())
    }

    fn read_by_id(txn: &Transaction, _id: Self::Id) -> crate::Result<Option<Self>> {
        let s: String = txn.query_row("SELECT head from Head;", (), |row| row.get(0))?;
        let head = serde_json::from_str(&s).map_err(|e| anyhow!("Invalid head string: {}", s))?;
        Ok(head)
    }

    fn persist(&self, txn: &Transaction) -> crate::Result<()> {
        txn.execute("DELETE FROM Head;", ())?;
        let s = serde_json::to_string(self)?;
        txn.execute("INSERT INTO Head (head) values (?1);", params![s])?;
        Ok(())
    }
}
