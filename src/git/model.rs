//! This module implements the interface between gitqlite models and the sqlite database.
//! Hash compute algorithm:
//! 1. The hash of a glob (glob_id) is the SHA256 of the file content.
//! 2. The hash of a tree (tree_id) is the SHA256 of the tree data.
//! 3. The hash of a commit (commit_id) is the SHA256 of the content built by joining all the fields with "\n".

use anyhow::{anyhow, Context};
use serde::{de::Visitor, Deserialize, Serialize};
use sha1::{self, Digest};
use std::{fmt, fs, path::Path};

use rusqlite::{
    params,
    types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef},
    Connection, OptionalExtension, ToSql,
};

use super::constants;

/// HEAD points to a ref
pub const CREATE_HEAD_TABLE: &str = "CREATE TABLE Head (ref_name TEXT NOT NULL);";
/// Ref points to a commit
pub const CREATE_REF_TABLE: &str =
    "CREATE TABLE Refs (ref_name TEXT PRIMARY KEY, commit_id BLOB NOT NULL);";
/// Commit points to a tree and contains a set of metadata
/// Note: parent_id is empty for the root commit, and for other commits,
/// parent_id is a list of sha1 hash blobs stored side by side, and we leverage the fact that sha1 hashes are always 8-bytes long to delimit them.
pub const CREATE_COMMIT_TABLE: &str = "CREATE TABLE Commits (commit_id BLOB PRIMARY KEY, tree_id TEXT NOT NULL, parent_ids BLOB NOT NULL, author_name TEXT NOT NULL, author_email TEXT NOT NULL, committer_name TEXT NOT NULL, committer_email TEXT NOT NULL, message TEXT NOT NULL);";
/// Tree points to a list of other trees (subdirectories) and blobs (file contents) and maintains their symbolic names
/// This data is encoded as a newline-separated text following the original git file format, where each line is of format
/// <file_mode> <file_type[blob|tree]> <object_id[tree_id|blob_id]> <file_name>
pub const CREATE_TREE_TABLE: &str =
    "CREATE TABLE Trees (tree_id TEXT PRIMARY KEY, data TEXT NOT NULL);";
/// Blob stores actual file content
pub const CREATE_BLOB_TABLE: &str = "CREATE TABLE Blobs (blob_id TEXT, data BLOB NOT NULL);";

// Read queries
pub const READ_BLOB_FOR_ID: &str = "SELECT blob_id, data FROM Blobs WHERE blob_id = ?1";
pub const READ_TREE_FOR_ID: &str = "SELECT tree_id, data FROM Trees WHERE tree_id = ?1";
pub const READ_COMMIT_FOR_ID: &str = "SELECT commit_id, tree_id, parent_ids, author_name, author_email, committer_name, committer_email, message FROM Commits WHERE commit_id = ?1";
pub const READ_REF_FOR_NAME: &str = "SELECT ref_name, commit_id FROM Refs WHERE ref_name = ?1";

// Write queries
pub const INSERT_BLOB: &str = "INSERT OR IGNORE INTO Blobs (blob_id, data) VALUES (?1, ?2);";
pub const INSERT_TREE: &str = "INSERT OR IGNORE INTO Trees (tree_id, data) VALUES (?1, ?2);";
pub const INSERT_COMMIT: &str = "INSERT OR IGNORE INTO Commits (commit_id, tree_id, parent_ids, author_name, author_email, committer_name, committer_email, message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);";
pub const INSERT_OR_REPLACE_REF: &str =
    "INSERT OR REPLACE INTO Refs (ref_name, commit_id) VALUES (?1, ?2);";

/// Generic trait describing any git object that could be hashed and get an ID for.
pub trait Hashable {
    fn hash(&self, sha: sha1::Sha1) -> Sha1Id;
}

impl<T> Hashable for Blob<T> {
    fn hash(&self, mut sha: sha1::Sha1) -> Sha1Id {
        // The hash of the glob is just the hash of the content
        sha.update(&self.data);
        let result = sha.finalize();
        Sha1Id(result.into())
    }
}

impl<T> Hashable for Tree<T> {
    fn hash(&self, mut sha: sha1::Sha1) -> Sha1Id {
        // The hash of the tree is the hash of all the tree entries in the format
        // <mode> <type> <id> <name>
        // concatenated with "\n"
        let text = self.encode_entries();
        sha.update(&text);

        let result = sha.finalize();
        Sha1Id(result.into())
    }
}

impl<T> Hashable for Commit<T> {
    fn hash(&self, mut sha: sha1::Sha1) -> Sha1Id {
        // the hash of the commit is the hash of all the fields concatednated in the form
        // <tree_id>
        // <parent_id>
        // ...
        // <author_name> <author_email>
        // <committer_name> <committer_email>
        // [empty line]
        // <message>
        // [empty line]

        sha.update(self.tree_id.0);
        sha.update("\n");

        for parent in &self.parent_ids {
            sha.update(parent.0);
            sha.update("\n");
        }

        sha.update(&self.author_name);
        sha.update(" ");
        sha.update(&self.author_email);
        sha.update("\n");

        sha.update(&self.committer_name);
        sha.update(" ");

        sha.update(&self.committer_email);
        sha.update("\n\n");

        sha.update(&self.message);
        sha.update("\n");

        let result = sha.finalize();
        Sha1Id(result.into())
    }
}

/// The models provded in this module like [`Commit`] and [`Blob`] have two possible states:
/// 1. Freshly constructed from the staging area -> No Id yet
/// 2. Stored in the gitqlite database -> Has a valid hash as Id
/// The [`IdType`] trait generalizes over the states.
pub trait IdType<T>: Copy + fmt::Display {
    type Id: PartialEq + Eq;

    #[allow(dead_code)]
    /// Returns the inner ID.
    fn id(self) -> Self::Id;
}

/// NoId signals that an object is just constructed and is not hashed and id-ed yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoId;

impl fmt::Display for NoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<NO ID>")
    }
}

impl<T> IdType<T> for NoId {
    type Id = std::convert::Infallible;

    fn id(self) -> Self::Id {
        unreachable!("You mustn't try to access non-IDs.");
    }
}

/// The canonical ID type used for all git objects, which is a SHA1 hash byte array
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sha1Id([u8; 20]);

impl fmt::Display for Sha1Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join("")
        )
    }
}

impl Serialize for Sha1Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as the hex string representation
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Sha1Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Sha1IdVisitor;
        impl<'v> Visitor<'v> for Sha1IdVisitor {
            type Value = Sha1Id;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("A hex string of length 40")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match Sha1Id::try_from(v) {
                    Ok(sha1_id) => Ok(sha1_id),
                    Err(e) => Err(E::custom(format!("{}", e))),
                }
            }
        }

        let s = deserializer.deserialize_str(Sha1IdVisitor)?;
        Ok(s)
    }
}

impl TryFrom<&str> for Sha1Id {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 40 {
            return Err(anyhow!("Invalid sha1 string: {}", value));
        }

        let mut bytes: [u8; 20] = [0; 20];

        for idx in (0..40).step_by(2) {
            let byte =
                u8::from_str_radix(&value[idx..idx + 2], 16).context("Converting str to Sha1Id")?;
            bytes[idx / 2] = byte;
        }

        Ok(Sha1Id(bytes))
    }
}

impl TryFrom<Vec<u8>> for Sha1Id {
    type Error = anyhow::Error;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        let Ok(bytes) = value.try_into() else {
            return Err(anyhow!("Byte is not valid Sha1"));
        };
        Ok(Sha1Id(bytes))
    }
}

impl IdType<Sha1Id> for Sha1Id {
    type Id = Sha1Id;

    fn id(self) -> Self::Id {
        self
    }
}

impl FromSql for Sha1Id {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let inner = <[u8; 20] as FromSql>::column_result(value)?;
        Ok(Sha1Id(inner))
    }
}

impl ToSql for Sha1Id {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Head {
    Branch(String),
    Commit(Sha1Id),
}

impl Head {
    pub fn get_current(gitqlite_home: impl AsRef<Path>) -> crate::Result<Head> {
        let head_path = gitqlite_home.as_ref().join(constants::HEAD_FILE_PREFIX);
        let f = fs::File::open(&head_path)?;
        let head = serde_json::from_reader(f)?;
        Ok(head)
    }

    pub fn persist(&self, gitqlite_home: impl AsRef<Path>) -> crate::Result<()> {
        let head_path = gitqlite_home.as_ref().join(constants::HEAD_FILE_PREFIX);
        let f = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&head_path)?;
        serde_json::to_writer(f, self)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Ref {
    pub name: String,
    pub commit_id: Sha1Id,
}

impl Ref {
    /// Read a reference from the database with the given name.
    /// Note that this is expected to return None in some cases, e.g., when the repository
    /// is just created with no commit yet, the HEAD will point to refs/head/main but the reference
    /// would not exist yet
    pub fn read_from_conn_with_name(
        conn: &Connection,
        name: impl AsRef<str>,
    ) -> crate::Result<Option<Ref>> {
        conn.query_row_and_then(READ_REF_FOR_NAME, [name.as_ref()], |row| {
            let ref_name = row.get(0)?;
            let commit_id = row.get(1)?;
            Ok(Ref {
                name: ref_name,
                commit_id,
            })
        })
        .optional()
        .map_err(anyhow::Error::from)
    }

    pub fn persist_or_update(&self, conn: &Connection) -> crate::Result<()> {
        conn.execute(INSERT_OR_REPLACE_REF, params![self.name, self.commit_id])?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Commit<ID> {
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

    pub fn with_id(self, id: Sha1Id) -> Commit<Sha1Id> {
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

impl Commit<Sha1Id> {
    pub fn read_from_conn_with_id(conn: &Connection, id: Sha1Id) -> crate::Result<Commit<Sha1Id>> {
        conn.query_row_and_then(READ_COMMIT_FOR_ID, [id], |row| {
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
    }

    pub fn persist(&self, conn: &Connection) -> crate::Result<()> {
        let mut parent_ids: Vec<u8> = Vec::with_capacity(self.parent_ids.len() * 20);
        for parent_id in &self.parent_ids {
            parent_ids.extend(parent_id.0.iter());
        }

        conn.execute(
            INSERT_COMMIT,
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

#[derive(Debug, PartialEq, Eq)]
pub struct Tree<ID> {
    pub tree_id: ID,
    pub entries: Vec<TreeEntry>,
}

impl<ID> Tree<ID> {
    fn encode_entries(&self) -> String {
        use std::fmt::Write;
        let mut buffer = String::new();
        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                writeln!(&mut buffer).unwrap();
            }
            let type_ = match entry.type_ {
                TreeEntryType::Blob => "blob",
                TreeEntryType::Tree => "tree",
            };
            write!(
                &mut buffer,
                "{} {} {} {}",
                entry.mode, type_, entry.id, entry.name
            )
            .unwrap();
        }

        buffer
    }
}

impl Tree<NoId> {
    pub fn new(entries: Vec<TreeEntry>) -> Tree<NoId> {
        Tree {
            tree_id: NoId,
            entries,
        }
    }

    pub fn with_id(self, id: Sha1Id) -> Tree<Sha1Id> {
        Tree {
            tree_id: id,
            entries: self.entries,
        }
    }
}

impl Tree<Sha1Id> {
    pub fn read_from_conn_with_id(conn: &Connection, id: Sha1Id) -> crate::Result<Tree<Sha1Id>> {
        conn.query_row_and_then(READ_TREE_FOR_ID, [id], |row| {
            let tree_id = row.get(0)?;
            let tree_data: String = row.get(1)?;

            let mut entries = vec![];

            for line in tree_data.split('\n') {
                // line format: <file_mode> <file_type[blob|tree]> <object_id[tree_id|blob_id]> <file_name>
                let mut split = line.split(' ');
                let mode = split.next().unwrap().to_string();
                let type_ = match split.next().unwrap() {
                    "blob" => TreeEntryType::Blob,
                    "tree" => TreeEntryType::Tree,
                    _ => unreachable!(),
                };

                let object_id = split.next().unwrap().try_into().unwrap();
                let name = split.next().unwrap().to_string();

                entries.push(TreeEntry {
                    type_,
                    id: object_id,
                    mode,
                    name,
                })
            }

            Ok(Tree { tree_id, entries })
        })
    }

    pub fn persist(&self, conn: &Connection) -> crate::Result<()> {
        let data = self.encode_entries();
        conn.execute(INSERT_TREE, params![self.tree_id, data])?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TreeEntryType {
    Blob,
    Tree,
}

impl fmt::Display for TreeEntryType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TreeEntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TreeEntryType::Blob => "blob",
            TreeEntryType::Tree => "tree",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TreeEntry {
    pub type_: TreeEntryType,
    pub id: Sha1Id,
    // ? We don't currently use mode yet, and haven't settled on how mode is going to be represented
    pub mode: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Blob<ID> {
    pub blob_id: ID,
    pub data: Vec<u8>,
}

impl Blob<Sha1Id> {
    pub fn read_from_conn_with_id(conn: &Connection, id: Sha1Id) -> crate::Result<Blob<Sha1Id>> {
        conn.query_row_and_then(READ_BLOB_FOR_ID, [id], |row| {
            let blob_id = row.get(0)?;
            let data = row.get(1)?;
            Ok(Blob { blob_id, data })
        })
    }

    pub fn persist(&self, conn: &Connection) -> crate::Result<()> {
        conn.execute(INSERT_BLOB, params![&self.blob_id, &self.data])?;
        Ok(())
    }
}

impl Blob<NoId> {
    pub fn new(data: Vec<u8>) -> Blob<NoId> {
        Self {
            blob_id: NoId,
            data,
        }
    }

    /// Invariant: Ensure that the id is computed from [`<Blob as Hashable>::hash`]
    pub fn with_id(self, id: Sha1Id) -> Blob<Sha1Id> {
        Blob {
            blob_id: id,
            data: self.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::params;

    use super::*;

    #[test]
    fn test_read_ref_none() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_REF_TABLE, ()).unwrap();
        let r = Ref::read_from_conn_with_name(&conn, "ABNA").unwrap();
        assert_eq!(None, r)
    }

    #[test]
    fn test_read_blob() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_BLOB_TABLE, ()).unwrap();

        let blob_id = "5baa61e4c9b93f3f0682250b6cf8331b7ee68fd8"
            .try_into()
            .unwrap();
        let data = [1u8, 2, 3, 4, 5];

        conn.execute(
            "INSERT INTO Blobs (blob_id, data) VALUES (?1, ?2)",
            params![blob_id, &data],
        )
        .unwrap();

        let blob = Blob::read_from_conn_with_id(&conn, blob_id).unwrap();

        assert_eq!(&data[..], &blob.data[..]);
    }

    #[test]
    fn test_read_tree() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_TREE_TABLE, ()).unwrap();

        let tree_id: Sha1Id = "b4c57b065cf9a5e83370b6f08759c0867a7fd523"
            .try_into()
            .unwrap();
        let entries = "100100 blob da39a3ee5e6b4b0d3255bfef95601890afd80709 file1
100100 tree 2fd4e1c67a2d28fced849ee1bb76e7391b93eb12 file2";

        let expected_entries = vec![
            TreeEntry {
                type_: TreeEntryType::Blob,
                id: "da39a3ee5e6b4b0d3255bfef95601890afd80709"
                    .try_into()
                    .unwrap(),
                mode: "100100".to_string(),
                name: "file1".to_string(),
            },
            TreeEntry {
                type_: TreeEntryType::Tree,
                id: "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12"
                    .try_into()
                    .unwrap(),
                mode: "100100".to_string(),
                name: "file2".to_string(),
            },
        ];

        let expected_tree = Tree {
            tree_id,
            entries: expected_entries,
        };

        conn.execute(
            "INSERT INTO Trees (tree_id, data) VALUES (?1, ?2);",
            params![tree_id, entries],
        )
        .unwrap();

        let tree = Tree::read_from_conn_with_id(&conn, tree_id).unwrap();

        assert_eq!(expected_tree, tree);
    }

    #[test]
    fn test_read_commit() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_COMMIT_TABLE, ()).unwrap();

        let commit_id = "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"
            .try_into()
            .unwrap();
        let tree_id = "3ca25ae354e192b26879f651a51d92aa8a34d8d3"
            .try_into()
            .unwrap();
        let parent_ids = vec![];
        let author_name = "eikasia30";
        let author_email = "eikasia30@gmail.com";
        let committer_name = "eikasia30";
        let committer_email = "eikasia30@gmail.com";
        let message = "test";

        conn.execute("INSERT INTO Commits (commit_id, tree_id, parent_ids, author_name, author_email, committer_name, committer_email, message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);", params![
          commit_id,
          tree_id,
          Vec::<u8>::new(),
          author_name,
          author_email,
          committer_name,
          committer_email,
          message
        ]).unwrap();

        let commit = Commit::read_from_conn_with_id(&conn, commit_id).unwrap();

        let expected_commit = Commit {
            commit_id: commit_id,
            tree_id: tree_id,
            parent_ids,
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            committer_name: committer_name.to_string(),
            committer_email: committer_email.to_string(),
            message: message.to_string(),
        };

        assert_eq!(expected_commit, commit);
    }

    #[test]
    fn test_hash_blob() {
        let data = b"daslkdjaslkdjaslkjdaslkALJKDSlkjsadclje";

        // Two blobs with the same data should hash to the same ID
        let blob1 = Blob::new(data.to_vec());
        let blob2 = Blob::new(data.to_vec());

        let blob1_id = blob1.hash(sha1::Sha1::new());
        let blob2_id = blob2.hash(sha1::Sha1::new());
        assert_eq!(blob1_id, blob2_id)
    }

    #[test]
    fn test_insert_same_blob() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(CREATE_BLOB_TABLE, ()).unwrap();

        let data = b"AAASdlkaSJdkljwehsajlkfdewqjklfdewqjlkwl";
        let blob1 = Blob::new(data.to_vec());
        let id = blob1.hash(sha1::Sha1::new());
        let blob1 = blob1.with_id(id);

        let blob2 = Blob::new(data.to_vec());
        let id = blob2.hash(sha1::Sha1::new());
        let blob2 = blob2.with_id(id);

        assert!(blob1.persist(&conn).is_ok());
        assert!(blob2.persist(&conn).is_ok());
    }
}
