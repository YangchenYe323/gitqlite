//! # Git Configuration Parser
//!
//! This module provides functionality to parse and manage Git configuration files.
//! It supports reading system, global, and local Git configurations, as well as
//! handling the `[include]` directive within these configurations.
//!
//! The main struct `GitConfig` represents a complete Git configuration, combining
//! settings from multiple sources with appropriate precedence.
//!
//! ## Features
//!
//! - Reads system, global, and local Git configurations
//! - Supports the `[include]` directive for including additional config files
//! - Respects the precedence order: system < global < local
//! - Provides easy access to configuration values
//!
//! ## Usage
//!
//! ```rust
//! use gitqlite::repo::config::GitConfig;
//! use gitqlite::repo::config::ConfigSource;
//!
//! fn main() -> anyhow::Result<()> {
//!     // Pass in the path to gitqlite home directory
//!     let git_config = GitConfig::load("/path/to/repo/.gitqlite")?;
//!     
//!     if let Some(user_name) = git_config.get("user.name", ConfigSource::All)? {
//!         println!("Git user name: {}", user_name);
//!     }
//!
//!     Ok(())
//! }
//! ```

use anyhow::anyhow;
use ini::Ini;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// Default system config path on windows/unix platforms
#[cfg(target_os = "windows")]
const SYSTEM_CONFIG_PATH: &str = r#"c:/Program Files/Git/etc/gitconfig"#;
#[cfg(not(target_os = "windows"))]
const SYSTEM_CONFIG_PATH: &str = "/etc/gitconfig";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    System,
    Global,
    Local,
    All,
}

type ConfigInner = HashMap<String, HashMap<String, String>>;

/// [`GitConfig`] stores the in-memory snapshot of the git configuration, constructed from:
/// 1. Syetem git configuration (use GIT_SYSTEM_CONFIG environment variable to override the path)
/// 2. Global git configuration in $HOME/.gitconfig
/// 3. Repository local git configuration in $GITQLITE_DIR/config
#[derive(Debug, Clone)]
pub struct GitConfig {
    system_path: PathBuf,
    global_path: PathBuf,
    local_path: PathBuf,

    system_config: ConfigInner,
    global_config: ConfigInner,
    local_config: ConfigInner,
}

impl GitConfig {
    pub fn load(gitqlite_home: impl AsRef<Path>) -> crate::Result<Self> {
        // Load system config
        let system_path = if let Ok(system_config_path) = std::env::var("GIT_SYSTEM_CONFIG") {
            PathBuf::from(system_config_path)
        } else {
            // Default system config path
            PathBuf::from(SYSTEM_CONFIG_PATH)
        };

        // Load global config
        let home_dir = dirs::home_dir().ok_or(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Home directory not found",
        ))?;
        let global_path = home_dir.join(".gitconfig");

        // Load local config
        let local_path = gitqlite_home.as_ref().join("config");

        let system_config = GitConfig::load_config(&system_path)?;
        let global_config = GitConfig::load_config(&global_path)?;
        let local_config = GitConfig::load_config(&local_path)?;

        let config = GitConfig {
            system_path,
            global_path,
            local_path,
            system_config,
            global_config,
            local_config,
        };

        Ok(config)
    }

    pub fn get(&self, key: &str, source: ConfigSource) -> crate::Result<Option<&str>> {
        let (section, key) = key
            .split_once(".")
            .ok_or_else(|| anyhow!("Config key must be of form SECTION.KEY"))?;

        Ok(match source {
            ConfigSource::System => self.get_system_inner(section, key),
            ConfigSource::Global => self.get_global_inner(section, key),
            ConfigSource::Local => self.get_local_inner(section, key),
            ConfigSource::All => self.get_all_inner(section, key).map(|(val, _)| val),
        })
    }

    pub fn get_with_source(
        &self,
        key: &str,
        source: ConfigSource,
    ) -> crate::Result<Option<(&str, &Path)>> {
        let (section, key) = key
            .split_once(".")
            .ok_or_else(|| anyhow!("Config key must be of form SECTION.KEY"))?;
        Ok(match source {
            ConfigSource::System => self
                .get_system_inner(section, key)
                .map(|val| (val, self.system_path.as_path())),
            ConfigSource::Global => self
                .get_global_inner(section, key)
                .map(|val| (val, self.global_path.as_path())),
            ConfigSource::Local => self
                .get_local_inner(section, key)
                .map(|val| (val, self.local_path.as_path())),
            ConfigSource::All => self.get_all_inner(section, key),
        })
    }

    pub fn set(&mut self, key: &str, value: String, source: ConfigSource) -> crate::Result<()> {
        let (section, key) = key
            .split_once(".")
            .ok_or_else(|| anyhow!("Config key must be of form SECTION.KEY"))?;

        match source {
            ConfigSource::System => self.set_system_inner(section, key, value),
            ConfigSource::Global => self.set_global_inner(section, key, value),
            _ => self.set_local_inner(section, key, value),
        }
    }

    fn load_config(config_path: impl AsRef<Path>) -> crate::Result<ConfigInner> {
        let mut config = HashMap::new();
        let mut seen = HashSet::new();

        GitConfig::load_config_rec(&mut config, &mut seen, config_path.as_ref())?;

        Ok(config)
    }

    fn load_config_rec(
        config: &mut ConfigInner,
        seen: &mut HashSet<PathBuf>,
        config_path: &Path,
    ) -> crate::Result<()> {
        if seen.contains(config_path) {
            return Err(anyhow!("Config contains recursive include chain"));
        }
        seen.insert(config_path.to_path_buf());

        let ini = Ini::load_from_file(config_path).unwrap_or_default();

        for (section, properties) in ini.iter() {
            let section_name = section.unwrap_or("").to_string();
            if section_name == "include" {
                if let Some(path) = properties.get("path") {
                    let include_path = Path::new(path);
                    GitConfig::load_config_rec(config, seen, include_path)?;
                }
                continue;
            }

            let section_map = config
                .entry(section_name.clone())
                .or_insert_with(HashMap::new);

            for (key, value) in properties.iter() {
                section_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(())
    }

    fn get_all_inner(&self, section: &str, key: &str) -> Option<(&str, &Path)> {
        if let Some(val) = self.get_local_inner(section, key) {
            return Some((val, &self.local_path));
        }

        if let Some(val) = self.get_global_inner(section, key) {
            return Some((val, &self.global_path));
        }

        if let Some(val) = self.get_system_inner(section, key) {
            return Some((val, &self.system_path));
        }

        None
    }

    fn get_system_inner(&self, section: &str, key: &str) -> Option<&str> {
        if let Some(section_map) = self.system_config.get(section) {
            if let Some(val) = section_map.get(key) {
                return Some(&*val);
            }
        }
        None
    }

    fn get_global_inner(&self, section: &str, key: &str) -> Option<&str> {
        if let Some(section_map) = self.global_config.get(section) {
            if let Some(val) = section_map.get(key) {
                return Some(&*val);
            }
        }
        None
    }

    fn get_local_inner(&self, section: &str, key: &str) -> Option<&str> {
        if let Some(section_map) = self.local_config.get(section) {
            if let Some(val) = section_map.get(key) {
                return Some(&*val);
            }
        }
        None
    }

    fn set_system_inner(&mut self, section: &str, key: &str, value: String) -> crate::Result<()> {
        let section_map = self
            .system_config
            .entry(section.to_string())
            .or_insert_with(HashMap::new);
        section_map.insert(key.to_string(), value.clone());
        let mut ini = Ini::load_from_file(&self.system_path)?;
        ini.set_to(Some(section), key.to_string(), value);
        ini.write_to_file(&self.system_path)?;
        Ok(())
    }

    fn set_global_inner(&mut self, section: &str, key: &str, value: String) -> crate::Result<()> {
        let section_map = self
            .global_config
            .entry(section.to_string())
            .or_insert_with(HashMap::new);
        section_map.insert(key.to_string(), value.clone());
        let mut ini = Ini::load_from_file(&self.global_path)?;
        ini.set_to(Some(section), key.to_string(), value);
        ini.write_to_file(&self.global_path)?;
        Ok(())
    }

    fn set_local_inner(&mut self, section: &str, key: &str, value: String) -> crate::Result<()> {
        let section_map = self
            .local_config
            .entry(section.to_string())
            .or_insert_with(HashMap::new);
        section_map.insert(key.to_string(), value.clone());
        let mut ini = Ini::load_from_file(&self.local_path).unwrap_or_default();
        ini.set_to(Some(section), key.to_string(), value);
        ini.write_to_file(&self.local_path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::repo::config::{self};

    use super::GitConfig;

    #[test]
    fn test_local_config() {
        let dir = tempdir().unwrap();
        let mut config = GitConfig::load(dir.path()).unwrap();

        // section.key does not exists now
        assert_eq!(
            None,
            config
                .get("section.key", config::ConfigSource::Local)
                .unwrap()
        );

        // set local
        config
            .set(
                "section.key",
                "value".to_string(),
                config::ConfigSource::Local,
            )
            .unwrap();

        println!("{:?}", config);
        assert_eq!(
            Some("value"),
            config
                .get("section.key", config::ConfigSource::Local)
                .unwrap()
        );

        // Now get a new config which should contains the new key
        let config = GitConfig::load(dir.path()).unwrap();
        assert_eq!(
            Some("value"),
            config
                .get("section.key", config::ConfigSource::Local)
                .unwrap()
        );
    }
}
