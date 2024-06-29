//! This module provides actual implementations of the git operations.

use std::{fs, path::Path};

use anyhow::Context;
use ini::Ini;
use model::{
    CREATE_BLOB_TABLE, CREATE_COMMIT_TABLE, CREATE_HEAD_TABLE, CREATE_REF_TABLE, CREATE_TREE_TABLE,
};
use rusqlite::Connection;

use crate::cli::InitArgs;

mod model {
    //! This module implements the interface between gitqlite models and the sqlite database.
    //! Hash compute algorithm:
    //! 1. The hash of a glob (glob_id) is the SHA256 of the file content.
    //! 2. The hash of a tree (tree_id) is the SHA256 of the tree data.
    //! 3. The hash of a commit (commit_id) is the SHA256 of the content built by joining all the fields with "\n".

    /// HEAD points to a ref
    pub const CREATE_HEAD_TABLE: &str = "CREATE TABLE Head (ref_name TEXT);";
    /// Ref points to a commit
    pub const CREATE_REF_TABLE: &str =
        "CREATE TABLE Refs (ref_name TEXT PRIMARY KEY, commit_id TEXT);";
    /// Commit points to a tree and contains a set of metadata
    pub const CREATE_COMMIT_TABLE: &str = "CREATE TABLE Commits (commit_id TEXT PRIMARY KEY, tree_id TEXT, author_name TEXT, author_email TEXT, committer_name TEXT, committer_email TEXT, message TEXT);";
    /// Tree points to a list of other trees (subdirectories) and blobs (file contents) and maintains their symbolic names
    /// This data is encoded as a newline-separated text following the original git file format, where each line is of format
    /// <file_mode> <file_type[blob|tree]> <object_id[tree_id|blob_id]> <file_name>
    pub const CREATE_TREE_TABLE: &str = "CREATE TABLE Trees (tree_id TEXT PRIMARY KEY, data TEXT);";
    /// Blob stores actual file content
    pub const CREATE_BLOB_TABLE: &str = "CREATE TABLE Blobs (blob_id TEXT, data BLOB);";

    #[derive(Debug)]
    pub struct Head(String);

    #[derive(Debug)]
    pub struct Ref {
        pub name: String,
        pub commit_id: String,
    }

    #[derive(Debug)]
    pub struct Commit {
        pub commit_id: String,
        pub tree_id: String,
        pub author_name: String,
        pub author_email: String,
        pub committer_name: String,
        pub committer_email: String,
        pubmessage: String,
    }

    #[derive(Debug)]
    pub struct Tree {
        pub tree_id: String,
    }

    #[derive(Debug)]
    pub enum TreeEntryType {
        Blob,
        Tree,
    }

    #[derive(Debug)]
    pub struct TreeEntry {
        pub type_: TreeEntryType,
        pub id: String,
        // ? We don't currently use mode yet, and haven't settled on how mode is going to be represented
        mode: String,
        pub name: String,
    }

    #[derive(Debug)]
    pub struct Blob {
        pub blob_id: String,
        pub data: Vec<u8>,
    }
}

mod constants {
    pub const GITQLITE_DIRECTORY_PREFIX: &str = ".gitqlite";
    pub const GITQLITE_DB_NAME: &str = "gitqlite.db";
}

pub fn do_init(_arg: InitArgs) -> crate::Result<()> {
    let pwd = std::env::current_dir()?;
    let gitqlite_home = pwd.join(constants::GITQLITE_DIRECTORY_PREFIX);

    let reinitialize = gitqlite_home.exists();

    if reinitialize {
        if gitqlite_home.is_dir() {
            fs::remove_dir_all(&gitqlite_home)?;
        } else {
            fs::remove_file(&gitqlite_home)?;
        }
    }

    fs::create_dir_all(&gitqlite_home)?;

    let db_path = gitqlite_home.join(constants::GITQLITE_DB_NAME);

    let conn = Connection::open(db_path)?;

    initialize_default_config(&gitqlite_home)?;
    initialize_gitqlite_tables(&conn)?;

    if reinitialize {
        println!(
            "Reinitialized existing Gitqlite repository in {}",
            gitqlite_home.display()
        );
    } else {
        println!(
            "Initialized Gitqlite repository in {}",
            gitqlite_home.display()
        )
    }

    Ok(())
}

fn initialize_gitqlite_tables(conn: &Connection) -> crate::Result<()> {
    conn.execute(CREATE_HEAD_TABLE, ())
        .context("Create Head table")?;
    conn.execute(CREATE_REF_TABLE, ())
        .context("Create Ref table")?;
    conn.execute(CREATE_COMMIT_TABLE, ())
        .context("Create Commit table")?;
    conn.execute(CREATE_TREE_TABLE, ())
        .context("Create Tree table")?;
    conn.execute(CREATE_BLOB_TABLE, ())
        .context("Create Blob table")?;
    Ok(())
}

fn initialize_default_config(gitqlite_home: impl AsRef<Path>) -> crate::Result<()> {
    let config = default_gitqlite_config();
    let config_path = gitqlite_home.as_ref().join("config");
    config
        .write_to_file(config_path)
        .context("Write config file")?;
    Ok(())
}

fn default_gitqlite_config() -> Ini {
    let mut conf = Ini::new();
    conf.with_section(Some("core"))
        .set("repositoryformatversion", "0")
        .set("filemode", "false")
        .set("bare", "false");
    conf
}
