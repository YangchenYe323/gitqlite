use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use sha1::Digest;

use super::{hash::Hashable, object::Object, IdType, NoId, Sha1Id};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit<ID: IdType<ID>> {
    pub commit_id: ID,
    pub tree_id: Sha1Id,
    pub parent_ids: Vec<Sha1Id>,
    pub author_name: String,
    pub author_email: String,
    pub committer_name: String,
    pub committer_email: String,
    pub message: String,
}

impl Commit<NoId> {
    pub fn new(
        tree_id: Sha1Id,
        parent_ids: Vec<Sha1Id>,
        author_name: String,
        author_email: String,
        committer_name: String,
        committer_email: String,
        message: String,
    ) -> Commit<NoId> {
        Commit {
            commit_id: NoId,
            tree_id,
            parent_ids,
            author_name,
            author_email,
            committer_name,
            committer_email,
            message,
        }
    }

    pub fn with_id(self) -> Commit<Sha1Id> {
        let sha = sha1::Sha1::new();
        let id = self.hash(sha);
        Commit {
            commit_id: id,
            tree_id: self.tree_id,
            parent_ids: self.parent_ids,
            author_name: self.author_name,
            author_email: self.author_email,
            committer_name: self.committer_name,
            committer_email: self.committer_email,
            message: self.message,
        }
    }
}

impl Hashable for Commit<NoId> {
    fn hash(&self, mut sha: sha1::Sha1) -> Sha1Id {
        let s = serde_json::to_vec(self).expect("Serialize commit failed");
        sha.update(s);
        Sha1Id(sha.finalize().into())
    }
}

impl Object for Commit<Sha1Id> {
    type Id = Sha1Id;

    fn type_(&self) -> super::object::ObjectType {
        super::object::ObjectType::Commit
    }

    fn create_table(txn: &Transaction) -> crate::Result<()> {
        txn.execute(
            "CREATE TABLE Commits (
                commit_id BLOB PRIMARY KEY,
                tree_id TEXT NOT NULL,
                parent_ids BLOB NOT NULL,
                author_name TEXT NOT NULL,
                author_email TEXT NOT NULL,
                committer_name TEXT NOT NULL,
                committer_email TEXT NOT NULL,
                message TEXT NOT NULL
            );",
            (),
        )?;
        Ok(())
    }

    fn read_by_id(txn: &Transaction, id: Sha1Id) -> crate::Result<Option<Commit<Sha1Id>>> {
        txn.query_row_and_then(
          "SELECT commit_id, tree_id, parent_ids, author_name, author_email, committer_name, committer_email, message FROM Commits WHERE commit_id = ?1;",
          [id],
          |row| {
            let commit_id = row.get(0)?;
            let tree_id = row.get(1)?;

            let parent_ids: Vec<Sha1Id> = row
                .get::<_, Vec<u8>>(2)?
                .chunks(20)
                .skip_while(|s| s.is_empty())
                .map(|s| {
                    let inner: [u8; 20] = s.try_into().unwrap();
                    Sha1Id(inner)
                })
                .collect();

            let author_name = row.get(3)?;
            let author_email = row.get(4)?;
            let committer_name = row.get(5)?;
            let committer_email = row.get(6)?;
            let message = row.get(7)?;
            Ok(Commit {
                commit_id,
                tree_id,
                parent_ids,
                author_name,
                author_email,
                committer_name,
                committer_email,
                message,
            })
        })
          .optional()
          .map_err(anyhow::Error::from)
    }

    fn persist(&self, txn: &Transaction) -> crate::Result<()> {
        let parent_ids: Vec<u8> = self
            .parent_ids
            .iter()
            .flat_map(|id| id.0.to_vec())
            .collect();

        // Use INSERT OR IGNORE because the same hash always means the same commit is already
        // present in the database.
        txn.execute(
            "INSERT OR IGNORE INTO Commits
             (commit_id, tree_id, parent_ids, author_name, author_email, committer_name, committer_email, message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            params![
                self.commit_id,
                self.tree_id,
                parent_ids,
                self.author_name,
                self.author_email,
                self.committer_name,
                self.committer_email,
                self.message
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_commit_creation_and_persistence() {
        let mut conn = Connection::open_in_memory().unwrap();
        let txn = conn.transaction().unwrap();

        Commit::create_table(&txn).unwrap();

        let tree_id = Sha1Id([1; 20]);
        let parent_ids = vec![Sha1Id([2; 20])];
        let commit = Commit::new(
            tree_id,
            parent_ids.clone(),
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "Jane Doe".to_string(),
            "jane@example.com".to_string(),
            "Test commit".to_string(),
        )
        .with_id();

        commit.persist(&txn).unwrap();

        let retrieved_commit = Commit::read_by_id(&txn, commit.commit_id).unwrap().unwrap();

        assert_eq!(commit.commit_id, retrieved_commit.commit_id);
        assert_eq!(commit.tree_id, retrieved_commit.tree_id);
        assert_eq!(commit.parent_ids, retrieved_commit.parent_ids);
        assert_eq!(commit.author_name, retrieved_commit.author_name);
        assert_eq!(commit.author_email, retrieved_commit.author_email);
        assert_eq!(commit.committer_name, retrieved_commit.committer_name);
        assert_eq!(commit.committer_email, retrieved_commit.committer_email);
        assert_eq!(commit.message, retrieved_commit.message);

        txn.commit().unwrap();
    }

    #[test]
    fn test_commit_hashing() {
        let tree_id = Sha1Id([1; 20]);
        let parent_ids = vec![Sha1Id([2; 20])];
        let commit1 = Commit::new(
            tree_id,
            parent_ids.clone(),
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "Jane Doe".to_string(),
            "jane@example.com".to_string(),
            "Test commit".to_string(),
        );

        let commit2 = Commit::new(
            tree_id,
            parent_ids,
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "Jane Doe".to_string(),
            "jane@example.com".to_string(),
            "Test commit".to_string(),
        );

        let commit3 = Commit::new(
            tree_id,
            vec![],
            "John Doe".to_string(),
            "john@example.com".to_string(),
            "Jane Doe".to_string(),
            "jane@example.com".to_string(),
            "Different message".to_string(),
        );

        assert_eq!(
            commit1.hash(sha1::Sha1::new()),
            commit2.hash(sha1::Sha1::new())
        );
        assert_ne!(
            commit1.hash(sha1::Sha1::new()),
            commit3.hash(sha1::Sha1::new())
        );
    }
}
