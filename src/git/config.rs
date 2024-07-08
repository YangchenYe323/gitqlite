//! This module contains functionalities for reading/writing git configurations
//! The structure of gitconfig is as follows:
//! There are three gitconfig files hirarchy when we read a value from the config, which are,
//! in increasing precedence:
//!
//! 1. System gitconfig, located at /etc/gitconfig on unix systems and C:\\Program Files\Git\etc\gitconfig on windows (Sorry, we don't consider windows XP)
//! 2. User global gitconfig, located at ~/.gitconfig
//! 3. Local gitconfig, located at PROJECT_ROOT/.gitqlite/.gitconfig
//!
//! And each config file is in ini config format
//!
//! git config <key> <value> -> write local gitconfig
//! git config --global <key> <value> -> write user global gitconfig
//! git config --system <key> <value> -> write system gitconfig

use std::path::Path;

use anyhow::{anyhow, Context};
use ini::Ini;

#[cfg(target_os = "windows")]
const SYSTEM_CONFIG_PATH: &str = r#"c:/Program Files/Git/etc/gitconfig"#;
#[cfg(not(target_os = "windows"))]
const SYSTEM_CONFIG_PATH: &str = "/etc/gitconfig";

pub fn initialize_default_config(gitqlite_home: impl AsRef<Path>) -> crate::Result<()> {
    let config = default_gitqlite_config();
    let config_path = gitqlite_home.as_ref().join("config");
    config
        .write_to_file(config_path)
        .context("Write config file")?;
    Ok(())
}

/// Get value from git config, returning the value and the origin
pub fn get_config_all(
    gitqlite_home: impl AsRef<Path>,
    config_key: &str,
) -> crate::Result<Option<(String, String)>> {
    if let Some(result) = get_local_config(gitqlite_home, config_key)? {
        return Ok(Some(result));
    }

    if let Some(result) = get_global_config(config_key)? {
        return Ok(Some(result));
    }

    get_system_config(config_key)
}

pub fn get_local_config(
    gitqlite_home: impl AsRef<Path>,
    config_key: &str,
) -> crate::Result<Option<(String, String)>> {
    let Some((section, key)) = config_key.split_once('.') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };
    let local_config_path = gitqlite_home.as_ref().join("config");

    get_config(local_config_path, section, key)
}

pub fn get_global_config(config_key: &str) -> crate::Result<Option<(String, String)>> {
    let Some((section, key)) = config_key.split_once('.') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };

    let home_dir = dirs::home_dir().unwrap();
    let user_global_config_path = home_dir.join(".gitconfig");

    get_config(user_global_config_path, section, key)
}

pub fn get_system_config(config_key: &str) -> crate::Result<Option<(String, String)>> {
    let Some((section, key)) = config_key.split_once('.') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };

    get_config(SYSTEM_CONFIG_PATH, section, key)
}

fn get_config(
    config_path: impl AsRef<Path>,
    section: &str,
    key: &str,
) -> crate::Result<Option<(String, String)>> {
    let path = config_path.as_ref();

    if let Ok(Some(value_from_local)) =
        Ini::load_from_file(path).map(|ini| ini.get_from(Some(section), key).map(str::to_string))
    {
        return Ok(Some((
            value_from_local,
            format!("file:{}", dunce::canonicalize(path).unwrap().display()),
        )));
    }

    Ok(None)
}

pub fn set_local_config(
    gitqlite_home: impl AsRef<Path>,
    config_key: &str,
    value: String,
) -> crate::Result<()> {
    let config_path = gitqlite_home.as_ref().join("config");

    let Some((section, key)) = config_key.split_once('.') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };

    set_config(config_path, section, key, value)
}

pub fn set_global_config(config_key: &str, value: String) -> crate::Result<()> {
    let Some((section, key)) = config_key.split_once(',') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };

    let home_dir = dirs::home_dir().unwrap();
    let user_global_config_path = home_dir.join(".gitconfig");

    set_config(user_global_config_path, section, key, value)
}

pub fn set_system_config(config_key: &str, value: String) -> crate::Result<()> {
    let Some((section, key)) = config_key.split_once('.') else {
        return Err(anyhow!("key does not contain a section: {}", config_key));
    };

    set_config(SYSTEM_CONFIG_PATH, section, key, value)
}

fn set_config(
    config_path: impl AsRef<Path>,
    section: &str,
    key: &str,
    value: String,
) -> crate::Result<()> {
    let path = config_path.as_ref();
    let mut config = Ini::load_from_file(path).unwrap_or_default();

    config.set_to(Some(section), key.to_string(), value.to_string());

    config.write_to_file(path)?;

    Ok(())
}

fn default_gitqlite_config() -> Ini {
    let mut conf = Ini::new();
    conf.with_section(Some("core"))
        .set("repositoryformatversion", "0")
        .set("filemode", "false")
        .set("bare", "false");
    conf
}
