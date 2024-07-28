use std::{
    collections::{BTreeMap, VecDeque},
    fs,
    io::Read,
    path::Path,
};

use anyhow::Ok;
use rusqlite::Connection;
use sha1::Digest;

use crate::{
    cli::StatusArgs,
    git::{
        constants,
        files::GitqliteFileMetadataExt,
        ignore::read_gitignore,
        index::{read_gitqlite_index, Index, IndexEntry},
        model::{self, Blob, Commit, Hashable, Head, Sha1Id, Tree, TreeEntryType},
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

    let head = Head::get_current(&gitqlite_home)?;

    // Print branch status
    print_status_branch(&head);
    println!();

    let index = index_map(read_gitqlite_index(&gitqlite_home)?);
    let head_tree_view = get_head_tree_view(&conn, head)?;

    // Print index/head diff (things to commit)
    match head_tree_view {
        Some(head_tree_view) => {
            print_diff_index_head(&index, &head_tree_view);
        }
        None => {
            println!("No commits yet");
            let dummy_tree_view = BTreeMap::new();
            print_diff_index_head(&index, &dummy_tree_view);
        }
    }

    println!();

    // Print index/work-tree diff (unstaged changes)
    print_diff_index_worktree(repo_root, index)?;
    println!();

    Ok(())
}

fn print_status_branch(head: &Head) {
    match head {
        Head::Branch(name) => {
            println!(
                "On branch {}",
                name.strip_prefix(constants::BRANCH_PREFIX).unwrap()
            )
        }
        Head::Commit(id) => {
            println!("Head detached at {}", id)
        }
    }
}

fn print_diff_index_head(
    index: &BTreeMap<String, IndexEntry>,
    head_tree_view: &BTreeMap<String, Sha1Id>,
) {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    for (name, entry) in index {
        if let Some(old_id) = head_tree_view.get(name) {
            if *old_id != entry.sha {
                modified.push(&*entry.name)
            }
        } else {
            added.push(&*entry.name)
        }
    }

    for (file, _) in head_tree_view {
        if !index.contains_key(file) {
            deleted.push(file.as_str());
        }
    }

    if added.is_empty() && modified.is_empty() && deleted.is_empty() {
        println!("no changes added to commit (use \"git add\" and/or \"git commit -a\")")
    } else {
        println!("Changes to be committed:");
        for add in added {
            println!("      added: {}", add);
        }
        for modify in modified {
            println!("      modified: {}", modify);
        }
        for delete in deleted {
            println!("      deleted: {}", delete);
        }
    }
}

fn print_diff_index_worktree(
    repo_root: impl AsRef<Path>,
    mut index: BTreeMap<String, IndexEntry>,
) -> crate::Result<()> {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut deleted = Vec::new();

    let git_home = repo_root.as_ref().join(".git");
    let gitqlite_home = repo_root
        .as_ref()
        .join(constants::GITQLITE_DIRECTORY_PREFIX);
    let gitignore = read_gitignore(repo_root.as_ref().to_path_buf())?;

    let mut queue = VecDeque::new();
    queue.push_back(repo_root.as_ref().to_path_buf().clone());

    while let Some(cur_directory) = queue.pop_front() {
        if cur_directory.starts_with(&gitqlite_home) || cur_directory.starts_with(&git_home) {
            continue;
        }

        for entry in fs::read_dir(&cur_directory)?.filter_map(Result::ok) {
            let path = entry.path();
            if gitignore.should_ignore(&path) {
                continue;
            }

            let rel_path = path
                .strip_prefix(repo_root.as_ref())?
                .to_string_lossy()
                .to_string();

            if path.is_dir() {
                queue.push_back(path);
                continue;
            }

            if !index.contains_key(&rel_path) {
                added.push(rel_path);
                continue;
            }

            let entry = index.get(&rel_path).unwrap();
            let mut f = fs::File::open(&path)?;
            let metadata = f.metadata()?;

            // Compare metadata first
            let actual_mtime = metadata.g_mtime();
            let is_modified = if actual_mtime != entry.mtime {
                let mut buffer = Vec::with_capacity(metadata.g_fsize() as usize);
                f.read_to_end(&mut buffer)?;
                let actual_hash = Blob::new(buffer).hash(sha1::Sha1::new());
                actual_hash != entry.sha
            } else {
                false
            };

            index.remove(&rel_path);
            if is_modified {
                modified.push(rel_path);
            }
        }
    }

    for (file, _) in &index {
        deleted.push(&**file);
    }

    if added.is_empty() && modified.is_empty() && deleted.is_empty() {
        println!("Nothing to commit")
    } else {
        println!("Changes not staged for commit:");
        for modify in modified {
            println!("      modified: {}", modify);
        }
        for delete in deleted {
            println!("      deleted: {}", delete);
        }
        println!("Untracked files:");
        for add in added {
            println!("      {}", add);
        }
    }

    Ok(())
}

fn get_head_tree_view(
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

/// Flatten a tree to a mapping from
/// full path relative to repo root -> SHA1 hash of the file
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

fn index_map(index: Index) -> BTreeMap<String, IndexEntry> {
    index
        .entries
        .into_iter()
        .map(|entry| (entry.name.clone(), entry))
        .collect()
}
