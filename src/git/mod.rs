//! This module provides actual implementations of the git operations.

use std::{fs, io::Read, path::Path};

use anyhow::{anyhow, Context};
use chrono::DateTime;
use constants::GITQLITE_INDEX_FILE;
use ignore::{check_gitignore, gitignore_read};
use index::{Index, ModeType};
use ini::Ini;
use model::{
    Blob, Commit, Hashable, Sha1Id, Tree, CREATE_BLOB_TABLE, CREATE_COMMIT_TABLE,
    CREATE_HEAD_TABLE, CREATE_REF_TABLE, CREATE_TREE_TABLE,
};
use rusqlite::Connection;
use sha1::Digest;
use utils::{find_gitqlite_root, get_gitqlite_connection};

use crate::cli::{CatFileArgs, CheckIgnoreArgs, HashObjectArgs, InitArgs, LsFilesArgs, ObjectType};

mod constants;
mod ignore;
mod index;
mod model;
mod utils;

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

pub fn do_cat_file(arg: CatFileArgs) -> crate::Result<()> {
    let CatFileArgs { type_, object } = arg;
    let conn = get_gitqlite_connection()?;

    let object_id = object.as_str().try_into()?;

    match type_ {
        crate::cli::ObjectType::Blob => print_blob(&conn, object_id),
        crate::cli::ObjectType::Tree => print_tree(&conn, object_id),
        crate::cli::ObjectType::Commit => print_commit(&conn, object_id),
    }
}

pub fn do_hash_object(arg: HashObjectArgs) -> crate::Result<()> {
    let HashObjectArgs { type_, write, file } = arg;
    let conn = get_gitqlite_connection()?;

    match type_ {
        ObjectType::Blob => {
            let blob = construct_blob_from_file(&file)?;
            if write {
                blob.persist(&conn)?;
            }
            println!("ID for {}: {}", file.display(), blob.blob_id);
        }
        _ => unimplemented!(),
    }

    Ok(())
}

pub fn do_ls_files(arg: LsFilesArgs) -> crate::Result<()> {
    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let index_file = repo_root.join(GITQLITE_INDEX_FILE);

    let file: Index = fs::File::open(index_file)
        .map(|f| serde_json::from_reader(f).expect("Malformed index.json file"))
        .unwrap_or_default();

    for entry in file.entries {
        println!("{}", entry.name);
        if arg.verbose {
            let file_type = match entry.mode_type {
                ModeType::Regular => "Regular File",
                ModeType::Symlink => "Symlink",
                ModeType::Gitlink => "Gitlink",
            };
            println!("    [{}] with perm {:o} ", file_type, entry.mode_perms);
            println!("    on blob: {}", entry.sha);

            let ctime = DateTime::from_timestamp_nanos(entry.ctime);
            let mtime = DateTime::from_timestamp_nanos(entry.mtime);
            println!(
                "    created on {}, last modified on {}",
                ctime.format("%Y-%m-%d %H:%M:%S.%f").to_string(),
                mtime.format("%Y-%m-%d %H:%M:%S.%f").to_string()
            );

            println!("    device {}, inode {}", entry.dev, entry.ino);

            println!("    user {}, group {}", entry.uid, entry.gid);

            println!(
                "    flags: stage={}, assume_valid={}",
                entry.flag_stage, entry.flag_assume_valid
            );
        }
    }

    Ok(())
}

pub fn do_check_ignore(arg: CheckIgnoreArgs) -> crate::Result<()> {
    let gitignore = gitignore_read()?;

    if check_gitignore(&gitignore, &arg.path) {
        println!("{}", arg.path.display());
    }

    Ok(())
}

fn construct_blob_from_file(path: impl AsRef<Path>) -> crate::Result<Blob<Sha1Id>> {
    let path = path.as_ref();

    if !path.is_file() {
        return Err(anyhow!(
            "Could not hash a non-file path to a blob: {}",
            path.display()
        ));
    }

    let data = {
        let mut f = fs::File::open(path)?;
        let mut buffer = Vec::with_capacity(1024);
        f.read_to_end(&mut buffer)?;
        buffer
    };

    let blob = Blob::new(data);

    let blob_id = blob.hash(sha1::Sha1::new());

    Ok(blob.with_id(blob_id))
}

fn print_blob(conn: &Connection, blob_id: Sha1Id) -> crate::Result<()> {
    let blob = Blob::read_from_conn_with_id(conn, blob_id)?;
    println!("{}", String::from_utf8_lossy(&blob.data));
    Ok(())
}

fn print_tree(conn: &Connection, tree_id: Sha1Id) -> crate::Result<()> {
    let tree = Tree::read_from_conn_with_id(conn, tree_id)?;

    for entry in &tree.entries {
        println!("{} {}    {}", entry.type_, entry.id, entry.name);
    }

    Ok(())
}

fn print_commit(conn: &Connection, commit_id: Sha1Id) -> crate::Result<()> {
    let commit = Commit::read_from_conn_with_id(conn, commit_id)?;
    println!("tree {}", commit.tree_id);
    for parent in &commit.parent_ids {
        println!("parent {}", parent);
    }
    println!("author {} <{}>", commit.author_name, commit.author_email);
    println!(
        "committer {} <{}>",
        commit.committer_name, commit.committer_email
    );
    println!();
    println!("{}", commit.message);
    println!();
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
