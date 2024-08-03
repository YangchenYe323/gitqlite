use crate::{
    cli::RmArgs,
    git::{
        model::Index,
        utils::{find_gitqlite_root, get_gitqlite_connection},
    },
};

pub fn do_rm(arg: RmArgs) -> crate::Result<()> {
    let RmArgs { path, cached } = arg;

    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let conn = get_gitqlite_connection()?;

    let mut index = Index::read_from_conn(&conn)?;
    if let Some(entry) = index.remove(&path, &repo_root, !cached)? {
        println!("rm {}", entry.name);
    }

    index.persist(&conn)?;

    Ok(())
}
