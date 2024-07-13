use crate::{
    cli::CheckIgnoreArgs,
    git::ignore::{check_gitignore, gitignore_read},
};

pub fn do_check_ignore(arg: CheckIgnoreArgs) -> crate::Result<()> {
    let gitignore = gitignore_read()?;

    if check_gitignore(&gitignore, &arg.path) {
        println!("{}", arg.path.display());
    }

    Ok(())
}
