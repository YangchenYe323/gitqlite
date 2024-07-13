use crate::git::{config, constants, utils::find_gitqlite_root};
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

    match (system, global, local) {
        (true, false, false) => {
            if let Some(value) = value {
                config::set_system_config(&name, value)
            } else {
                let value = config::get_system_config(&name)?;
                if let Some((value, origin)) = value {
                    if show_origin {
                        println!("{}    {}", origin, value);
                    } else {
                        println!("{}", value);
                    }
                }

                Ok(())
            }
        }
        (false, true, false) => {
            if let Some(value) = value {
                config::set_global_config(&name, value)
            } else {
                let value = config::get_global_config(&name)?;
                if let Some((value, origin)) = value {
                    if show_origin {
                        println!("{}    {}", origin, value);
                    } else {
                        println!("{}", value);
                    }
                }

                Ok(())
            }
        }
        (false, false, true) => {
            if let Some(value) = value {
                config::set_local_config(&gitqlite_home, &name, value)
            } else {
                let value = config::get_local_config(&gitqlite_home, &name)?;
                if let Some((value, origin)) = value {
                    if show_origin {
                        println!("{}    {}", origin, value);
                    } else {
                        println!("{}", value);
                    }
                }

                Ok(())
            }
        }
        (false, false, false) => {
            if let Some(value) = value {
                config::set_local_config(&gitqlite_home, &name, value)
            } else {
                let value = config::get_config_all(&gitqlite_home, &name)?;
                if let Some((value, origin)) = value {
                    if show_origin {
                        println!("{}    {}", origin, value);
                    } else {
                        println!("{}", value);
                    }
                }

                Ok(())
            }
        }
        _ => Err(anyhow!("error: only one config file at a time")),
    }
}
