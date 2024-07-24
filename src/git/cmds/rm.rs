use crate::{
    cli::RmArgs,
    git::{
        constants,
        index::{read_gitqlite_index, write_gitqlite_index},
        utils::find_gitqlite_root,
    },
};

pub fn do_rm(arg: RmArgs) -> crate::Result<()> {
    let RmArgs { path, cached } = arg;

    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let gitqlite_home = repo_root.join(constants::GITQLITE_DIRECTORY_PREFIX);

    let mut index = read_gitqlite_index(&gitqlite_home)?;
    if let Some(entry) = index.remove(&path, &repo_root, !cached)? {
        println!("rm {}", entry.name);
    }

    // Persist index to file
    write_gitqlite_index(&gitqlite_home, &index)?;

    Ok(())
}
