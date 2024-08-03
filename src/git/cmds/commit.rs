use std::{collections::HashMap, path::PathBuf};

use anyhow::anyhow;
use sha1::Digest;

use crate::{
    cli::CommitArgs,
    git::{
        config, constants,
        model::{
            Commit, Hashable, Head, Index, IndexEntry, Ref, Sha1Id, Tree, TreeEntry, TreeEntryType,
        },
        utils::{find_gitqlite_root, get_gitqlite_connection},
    },
};

#[derive(Debug)]
enum BlobOrTree {
    Blob(IndexEntry),
    Tree {
        tree: Tree<Sha1Id>,
        name: String,
        mode: String,
    },
}

impl BlobOrTree {
    pub fn name(&self) -> &str {
        match self {
            BlobOrTree::Blob(entry) => &entry.name,
            BlobOrTree::Tree {
                tree: _,
                name,
                mode: _,
            } => name,
        }
    }
}

pub fn do_commit(arg: CommitArgs) -> crate::Result<()> {
    let CommitArgs { message } = arg;
    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let gitqlite_home = repo_root.join(constants::GITQLITE_DIRECTORY_PREFIX);
    let conn = get_gitqlite_connection()?;

    let (user, _) = config::get_config_all(&gitqlite_home, "user.name")?
        .ok_or_else(|| anyhow!("Missing user.name in git config"))?;
    let (user_email, _) = config::get_config_all(&gitqlite_home, "user.email")?
        .ok_or_else(|| anyhow!("Missing user.email in git config"))?;

    let index = Index::read_from_conn(&conn)?;

    // trees stores relatvie path -> a list of index entries. We will iteratively build up
    // the git tree for the root repo
    let mut directory_entries: HashMap<PathBuf, Vec<BlobOrTree>> = HashMap::new();
    for entry in index.entries {
        let path = repo_root.join(&entry.name);
        let parent_dir = path.parent().unwrap();
        directory_entries
            .entry(parent_dir.to_path_buf())
            .or_default()
            .push(BlobOrTree::Blob(entry));
    }

    let mut trees = HashMap::new();

    // Sort directory by reverse length (subdirectories before their parents)
    let mut keys: Vec<PathBuf> = directory_entries.keys().cloned().collect();
    keys.sort_by_key(|path| -(path.to_string_lossy().len() as i32));

    for key in keys {
        // Create a tree object for this directory
        let entries = directory_entries.get_mut(&key).unwrap();
        // Sort tree entry by their name
        entries.sort_by(|e1, e2| e1.name().cmp(&e2.name()));
        let tree_entries = make_tree_entries(&entries);
        let tree = Tree::new(tree_entries);
        let tree_id = tree.hash(sha1::Sha1::new());
        let tree = tree.with_id(tree_id);
        tree.persist(&conn)?;
        trees.insert(key.clone(), tree.tree_id);

        if key == repo_root {
            continue;
        }
        let parent = key.parent().unwrap();
        directory_entries
            .get_mut(parent)
            .unwrap()
            .push(BlobOrTree::Tree {
                tree,
                name: key
                    .strip_prefix(&repo_root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                mode: "040000".to_string(),
            })
    }

    let root_tree = trees.get(&repo_root).unwrap();

    // Create commit
    // Get the current root commit
    let head = Head::read_from_conn(&conn)?;
    let root_commit = match &head {
        Head::Branch(branch) => Ref::read_from_conn_with_name(&conn, branch)?.map(|r| r.commit_id),
        Head::Commit(id) => Some(*id),
    };

    let parent_ids = if let Some(root_commit) = root_commit {
        vec![root_commit]
    } else {
        Vec::new()
    };

    let commit = Commit::new(
        *root_tree,
        parent_ids,
        user.clone(),
        user_email.clone(),
        user,
        user_email,
        message,
    );
    let commit_id = commit.hash(sha1::Sha1::new());
    let commit = commit.with_id(commit_id);
    commit.persist(&conn)?;

    // Update ref to the root commit
    match head {
        Head::Branch(name) => {
            let new_ref = Ref { name, commit_id };
            new_ref.persist_or_update(&conn)?;
        }
        Head::Commit(_) => {
            let new_head = Head::Commit(commit_id);
            new_head.persist(&conn)?;
        }
    }

    println!("Created new commit {}", commit.commit_id);

    Ok(())
}

/// Convert index entries to tree entries. Index entries must be sorted
fn make_tree_entries(index_entries: &[BlobOrTree]) -> Vec<TreeEntry> {
    index_entries
        .iter()
        .map(|entry| {
            let path = PathBuf::from(entry.name());
            let filename = path.file_name().unwrap().to_str().unwrap().to_string();

            match entry {
                BlobOrTree::Blob(entry) => TreeEntry {
                    type_: TreeEntryType::Blob,
                    id: entry.sha,
                    mode: entry.mode_perms.to_string(),
                    name: filename,
                },
                BlobOrTree::Tree {
                    tree,
                    mode,
                    name: _,
                } => TreeEntry {
                    type_: TreeEntryType::Tree,
                    id: tree.tree_id,
                    mode: mode.clone(),
                    name: filename,
                },
            }
        })
        .collect()
}

// fn get_root_commit_id(conn: &Connection, head: Head) -> crate::Result<Option<Sha1Id>> {
//   let root_commit_id = match head {
//       Head::Branch(branch) => {

//       }
//   };
// }
