use std::fs;
use std::path::Path;

use anyhow::Context;
use rusqlite::Connection;

use crate::cli::InitArgs;
use crate::git::config::{self, initialize_default_config};
use crate::git::index::{write_gitqlite_index, Index};
use crate::git::model::Head;
use crate::git::{constants, model};

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
    initialize_head(&gitqlite_home)?;
    write_gitqlite_index(&gitqlite_home, &Index::default())?;

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
    conn.execute(model::CREATE_HEAD_TABLE, ())
        .context("Create Head table")?;
    conn.execute(model::CREATE_REF_TABLE, ())
        .context("Create Ref table")?;
    conn.execute(model::CREATE_COMMIT_TABLE, ())
        .context("Create Commit table")?;
    conn.execute(model::CREATE_TREE_TABLE, ())
        .context("Create Tree table")?;
    conn.execute(model::CREATE_BLOB_TABLE, ())
        .context("Create Blob table")?;
    Ok(())
}

fn initialize_head(gitqlite_home: impl AsRef<Path>) -> crate::Result<()> {
    let (default_branch, _) = config::get_config_all(gitqlite_home.as_ref(), "init.defaultBranch")?
        .expect("Fail to retrieve default branch, please check your system gitconfig");
    let full_branch_name = format!("{}{}", constants::BRANCH_PREFIX, default_branch);

    let head = Head::Branch(full_branch_name);
    head.persist(gitqlite_home)
}
