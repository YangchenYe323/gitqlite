use crate::{
    cli::CheckIgnoreArgs,
    git::{ignore::read_gitignore, utils::find_gitqlite_root},
};

pub fn do_check_ignore(arg: CheckIgnoreArgs) -> crate::Result<()> {
    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let gitignore = read_gitignore(repo_root)?;

    if  gitignore.should_ignore(&arg.path) {
        println!("{}", arg.path.display());
    }

    Ok(())
}
