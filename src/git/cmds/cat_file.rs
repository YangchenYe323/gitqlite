use rusqlite::Connection;

use crate::{
    cli::CatFileArgs,
    git::{
        model::{Blob, Commit, Sha1Id, Tree},
        utils::get_gitqlite_connection,
    },
};

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
