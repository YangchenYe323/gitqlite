use std::fs;

use anyhow::anyhow;

use crate::git::constants::GITQLITE_DIRECTORY_PREFIX;
use crate::git::files::GitqliteFileMetadataExt;
use crate::git::ignore::read_gitignore;
use crate::git::model::{Index, IndexEntry, ModeType};
use crate::git::utils::get_gitqlite_connection;
use crate::{cli::AddArgs, git::utils::find_gitqlite_root};

use super::hash_object::construct_blob_from_file;

pub fn do_add(arg: AddArgs) -> crate::Result<()> {
    let AddArgs { path } = arg;

    let path = if path.is_absolute() {
        path
    } else {
        dunce::canonicalize(path)?
    };

    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let conn = get_gitqlite_connection()?;
    let gitqlite_home = repo_root.join(GITQLITE_DIRECTORY_PREFIX);
    let ignore = read_gitignore(gitqlite_home.clone())?;
    let mut index = Index::read_from_conn(&conn)?;

    if !path.starts_with(&repo_root) {
        return Err(anyhow!(
            "Path {} is not inside the current gitqlite repository",
            path.display()
        ));
    }

    if ignore.should_ignore(&path) {
        return Err(anyhow!(
            "Path {} is ignored by the repo .gitignore",
            path.display()
        ));
    }

    let rel_path = path.strip_prefix(&repo_root)?.to_string_lossy().to_string();

    // Create an index entry for the path
    // Step 1: create an object for the pathparams
    let blob = construct_blob_from_file(&path)?;
    blob.persist(&conn)?;

    // Step 2: Populate index entry from metadata
    let f = fs::File::open(&path)?;
    let metadata = f.metadata()?;
    index.entries = index
        .entries
        .into_iter()
        .filter(|entry| entry.name != rel_path)
        .collect();

    let entry = IndexEntry {
        ctime: metadata.g_ctime(),
        mtime: metadata.g_mtime(),
        dev: metadata.g_dev(),
        ino: metadata.g_ino(),
        mode_type: ModeType::Regular,
        mode_perms: metadata.g_mode_perms(),
        uid: metadata.g_uid(),
        gid: metadata.g_gid(),
        fsize: metadata.g_fsize(),
        sha: blob.blob_id,
        flag_assume_valid: false,
        flag_stage: 0,
        name: rel_path,
    };

    index.entries.push(entry);

    index.persist(&conn)?;
    Ok(())
}
