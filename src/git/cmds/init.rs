use std::fs;

use anyhow::Context;
use rusqlite::Connection;

use crate::cli::InitArgs;
use crate::git::config::initialize_default_config;
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
