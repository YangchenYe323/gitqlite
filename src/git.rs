//! This module provides actual implementations of the git operations.

use std::fs;

use rusqlite::Connection;

use crate::cli::InitArgs;

mod constants {
    pub const GITQLITE_DIRECTORY_PREFIX: &str = ".gitqlite";
    pub const GITQLITE_DB_NAME: &str = "gitqlite.db";
}

pub fn do_init(_arg: InitArgs) -> crate::Result<()> {
    let pwd = std::env::current_dir()?;
    let gitqlite_dir = pwd.join(constants::GITQLITE_DIRECTORY_PREFIX);

    let reinitialize = gitqlite_dir.exists();

    if reinitialize {
        if gitqlite_dir.is_dir() {
            fs::remove_dir_all(&gitqlite_dir)?;
        } else {
            fs::remove_file(&gitqlite_dir)?;
        }
    }

    fs::create_dir_all(&gitqlite_dir)?;

    let db_path = gitqlite_dir.join(constants::GITQLITE_DB_NAME);

    let _conn = Connection::open(db_path)?;

    if reinitialize {
        println!(
            "Reinitialized existing Gitqlite repository in {}",
            gitqlite_dir.display()
        );
    } else {
        println!(
            "Initialized Gitqlite repository in {}",
            gitqlite_dir.display()
        )
    }

    Ok(())
}
