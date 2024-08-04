use crate::git::{constants, utils::find_gitqlite_root};
use crate::repo::config::{self, GitConfig};
use anyhow::anyhow;

use crate::cli::ConfigArgs;

pub fn do_config(arg: ConfigArgs) -> crate::Result<()> {
    let ConfigArgs {
        name,
        value,
        show_origin,
        system,
        global,
        local,
    } = arg;

    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;
    let gitqlite_home = repo_root.join(constants::GITQLITE_DIRECTORY_PREFIX);

    let mut config = GitConfig::load(&gitqlite_home)?;

    let source = match (system, global, local) {
        (true, false, false) => config::ConfigSource::System,
        (false, true, false) => config::ConfigSource::Global,
        (false, false, true) => config::ConfigSource::Local,
        (false, false, false) => config::ConfigSource::All,
        _ => return Err(anyhow!("error: only one config file at a time")),
    };

    if let Some(value) = value {
        config.set(&name, value, source)
    } else {
        let value = config.get_with_source(&name, source)?;
        if let Some((value, origin)) = value {
            if show_origin {
                println!("{}    {}", origin.display(), value);
            } else {
                println!("{}", value);
            }
        }

        Ok(())
    }
}
