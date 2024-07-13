use std::{collections::BTreeMap, fs, path::Path};

use anyhow::Context;
use rusqlite::Connection;
use sha1::Sha1;

use crate::{
    cli::StatusArgs,
    git::{
        constants,
        model::{self, Blob, Commit, Sha1Id, Tree, TreeEntryType},
        utils::{find_gitqlite_root, get_gitqlite_connection},
    },
};

/// Status command does two things:
/// 1. Compare the content of the index file with the tree pointed to by the ref of HEAD, which
/// is shown in the files to be committed section.
/// 2. Compare the content of the index file with the current working directory, which is shown in
/// the files to be addeds. It also collects information about untracked files.
pub fn do_status(_arg: StatusArgs) -> crate::Result<()> {
    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let gitqlite_home = repo_root.join(constants::GITQLITE_DIRECTORY_PREFIX);
    let conn = get_gitqlite_connection()?;

    let head = get_current_head(&gitqlite_home)?;

    // Print branch status
    print_status_branch(&head);

    let head_tree_view = get_current_tree_view(&conn, head)?;

    // Print index/head diff (things to commit)
    match head_tree_view {
        Some(_head_tree_view) => {
            todo!()
        }
        None => {
            println!("No commits yet");
            Ok(())
        }
    }
}

enum Head {
    Branch(String),
    Commit(Sha1Id),
}

fn get_current_head(gitqlite_home: impl AsRef<Path>) -> crate::Result<Head> {
    let head_path = gitqlite_home.as_ref().join(constants::HEAD_FILE_PREFIX);
    let head_content = fs::read_to_string(head_path)?;
    if head_content.starts_with(constants::BRANCH_PREFIX) {
        return Ok(Head::Branch(head_content));
    }

    // Head is the hex format of an sha1 hash
    let id = Sha1Id::try_from(&*head_content).expect("Malformed HEAD file");
    Ok(Head::Commit(id))
}

fn print_status_branch(head: &Head) {
    match head {
        Head::Branch(name) => {
            println!("On branch {}", name.strip_prefix(constants::BRANCH_PREFIX).unwrap())
        }
        Head::Commit(id) => {
            println!("Head detached at {}", id)
        }
    }
}

fn get_current_tree_view(
    conn: &Connection,
    head: Head,
) -> crate::Result<Option<BTreeMap<String, Sha1Id>>> {
    let root_commit_id = match head {
        Head::Branch(branch_name) => {
            let Some(reference) = model::Ref::read_from_conn_with_name(conn, &branch_name)? else {
                return Ok(None);
            };

            reference.commit_id
        }
        Head::Commit(commit_id) => commit_id,
    };

    let root_commit = Commit::read_from_conn_with_id(conn, root_commit_id)?;
    tree_view(root_commit.tree_id, conn).map(Option::Some)
}

fn tree_view(tree_id: Sha1Id, conn: &Connection) -> crate::Result<BTreeMap<String, Sha1Id>> {
    let mut view = BTreeMap::new();

    // (current tree, prefix of file names in the current tree)
    let mut stack = Vec::with_capacity(32);
    stack.push((tree_id, "".to_string()));

    while let Some((cur_tree_id, prefix)) = stack.pop() {
        let cur_tree = Tree::read_from_conn_with_id(conn, cur_tree_id)?;
        for entry in cur_tree.entries {
            match entry.type_ {
                TreeEntryType::Blob => {
                    let blob_full_name = prefix.clone() + &format!("/{}", entry.name);
                    view.insert(blob_full_name, entry.id);
                }
                TreeEntryType::Tree => {
                    let next_prefix = prefix.clone() + &format!("/{}", entry.name);
                    stack.push((entry.id, next_prefix));
                }
            }
        }
    }

    Ok(view)
}
