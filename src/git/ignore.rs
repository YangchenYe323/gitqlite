//! This module implements parsing .gitignore files and applying the rules

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

use super::utils::find_gitqlite_root;

/// [`GitIgnore`] describes the whole git ignore structure of the current repository.
#[derive(Debug)]
pub struct GitIgnore {
    /// Scoped rules are .gitignore files that locate inside the repository, which only
    /// apply to paths under the respective sub-directory, and rules down the leaf override
    /// rules high up the tree.
    scoped: HashMap<PathBuf, Vec<IgnoreRule>>,

    /// Absolute rules are .gitignore files that locate in system configuration directories (e.g., ~/.config/.gitignore)
    /// They apply to all paths in the repository but are of lower priority.
    /// TODO: use it
    #[allow(dead_code)]
    absolute: Vec<Vec<IgnoreRule>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IgnoreRule {
    Exclude(String),
    Negate(String),
}

/// Parse one gitignore rule from the string
pub fn gitignore_parse_one(s: &str) -> Option<IgnoreRule> {
    let s = s.trim();

    let first_char = s.chars().next()?;

    match first_char {
        '!' => Some(IgnoreRule::Negate(s[1..].to_string())),
        '#' => None,
        '\\' => Some(IgnoreRule::Exclude(s[1..].to_string())),
        _ => Some(IgnoreRule::Exclude(s.to_string())),
    }
}

/// Parse a vector of rules from a data source (a .gitignore file, for example)
pub fn gitignore_parse(r: &mut impl Read) -> crate::Result<Vec<IgnoreRule>> {
    let mut rules = Vec::new();

    let buf_reader = BufReader::new(r);

    for line in buf_reader.lines() {
        let line = line?;
        if let Some(rule) = gitignore_parse_one(&line) {
            rules.push(rule);
        }
    }

    Ok(rules)
}

/// Read and build the whole gitignore structure of the current repository
pub fn gitignore_read() -> crate::Result<GitIgnore> {
    let repo_root = find_gitqlite_root(std::env::current_dir()?)?;

    let mut scoped = HashMap::new();

    // TODO: Implement absolute rules by looking at system configuration directories
    let absolute = Vec::new();

    // Run a dfs over the directory tree
    let mut stack = Vec::new();
    stack.push(repo_root);

    while let Some(current_dir) = stack.pop() {
        let gitignore_file = current_dir.join(".gitignore");

        // If there is a .gitignore file in the current directory
        if let Ok(mut file) = fs::File::open(gitignore_file) {
            let rules = gitignore_parse(&mut file)?;
            scoped.insert(current_dir.clone(), rules);
        }

        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path)
            }
        }
    }

    Ok(GitIgnore { scoped, absolute })
}

/// Return if the given target should be excluded given the gitignore configuration
pub fn check_gitignore(gitignore: &GitIgnore, target: impl AsRef<Path>) -> bool {
    let target = target.as_ref();

    let canonicalized_target = if target.is_relative() {
        // use dunce create to avoid \\? prefix on windows
        dunce::canonicalize(target).expect("Failed to canonicalize target")
    } else {
        target.to_path_buf()
    };

    if let Some(result) = check_ignore_scoped(&gitignore.scoped, canonicalized_target) {
        return result;
    }

    // TODO: implement absolute check
    false
}

/// Check if the target should be excluded according to the specific .gitignore file located in directory `path`
fn check_gitignore_one(
    dir: impl AsRef<Path>,
    rules: &[IgnoreRule],
    target: impl AsRef<Path>,
) -> Option<bool> {
    let dir = dir.as_ref();
    let target = target.as_ref();

    for rule in rules.iter().rev() {
        match rule {
            IgnoreRule::Exclude(pat) => {
                let full_pat = dir.join(pat);

                // TODO: we assume paths are valid UTF-8 string here. Could we drop the assumption?
                let Ok(paths) = glob::glob(full_pat.as_path().as_os_str().to_str().unwrap()) else {
                    log::warn!(
                        "Skipping malformed gitignore entry {}:{}",
                        dir.display(),
                        pat
                    );
                    // Skip malformed pattern
                    continue;
                };

                for expanded in paths.filter_map(Result::ok) {
                    if target.starts_with(&expanded) {
                        return Some(true);
                    }
                }
            }
            IgnoreRule::Negate(pat) => {
                let full_pat = dir.join(pat);

                // TODO: we assume paths are valid UTF-8 string here. Could we drop the assumption?
                let Ok(paths) = glob::glob(full_pat.as_path().as_os_str().to_str().unwrap()) else {
                    log::warn!(
                        "Skipping malformed gitignore entry {}:{}",
                        dir.display(),
                        pat
                    );
                    // Skip malformed pattern
                    continue;
                };

                for expanded in paths.filter_map(Result::ok) {
                    if target.starts_with(&expanded) {
                        return Some(false);
                    }
                }
            }
        }
    }

    // The current rules could not decide whether the target is included or not
    None
}

fn check_ignore_scoped(
    scoped: &HashMap<PathBuf, Vec<IgnoreRule>>,
    target: impl AsRef<Path>,
) -> Option<bool> {
    let target = target.as_ref();

    for dir in target.ancestors().skip(1) {
        if let Some(rules) = scoped.get(dir) {
            if let Some(result) = check_gitignore_one(dir, rules, target) {
                return Some(result);
            }
        }
    }

    None
}

impl GitIgnore {
    #[cfg(test)]
    pub(self) fn new_for_testing(
        scoped: HashMap<PathBuf, Vec<IgnoreRule>>,
        absolute: Vec<Vec<IgnoreRule>>,
    ) -> GitIgnore {
        GitIgnore { scoped, absolute }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_rules(text: &str) -> Vec<IgnoreRule> {
        let mut bytes = text.as_bytes();
        gitignore_parse(&mut bytes).unwrap()
    }

    #[test]
    fn test_parse_rules() {
        let gitignore_text = "
          *.txt      
          log/
          !log/abc.txt
        ";
        let rules = gen_rules(gitignore_text);

        assert_eq!(
            vec![
                IgnoreRule::Exclude("*.txt".to_string()),
                IgnoreRule::Exclude("log/".to_string()),
                IgnoreRule::Negate("log/abc.txt".to_string())
            ],
            rules
        )
    }

    #[test]
    fn test_check_rules_exclude() {
        let test_bed = tempfile::tempdir().unwrap();
        let test_dir_path = test_bed.path();
        let file_to_exclude = test_dir_path.join("file_to_exclude");

        fs::File::create(&file_to_exclude).unwrap();

        let rules = gen_rules(
            "
          file_to_exclude
        ",
        );

        let scoped = {
            let mut s = HashMap::new();
            s.insert(test_dir_path.to_path_buf(), rules);
            s
        };

        let absolute = vec![];

        let gitignore = GitIgnore::new_for_testing(scoped, absolute);

        assert!(check_gitignore(&gitignore, file_to_exclude))
    }

    #[test]
    fn test_check_rules_negate() {
        let test_bed = tempfile::tempdir().unwrap();
        let test_dir_path = test_bed.path();
        let file_to_include = test_dir_path.join("file_to_include.txt");
        let file_to_exclude = test_dir_path.join("file_to_exclude.txt");

        fs::File::create(&file_to_include).unwrap();
        fs::File::create(&file_to_exclude).unwrap();

        let rules = gen_rules(
            "
          *.txt
          !file_to_include.txt
        ",
        );

        let scoped = {
            let mut s = HashMap::new();
            s.insert(test_dir_path.to_path_buf(), rules);
            s
        };

        let absolute = vec![];

        let gitignore = GitIgnore::new_for_testing(scoped, absolute);

        assert!(check_gitignore(&gitignore, file_to_exclude));
        assert!(!check_gitignore(&gitignore, file_to_include));
    }

    #[test]
    fn test_check_rules_exclude_multi_levels() {
        let test_bed = tempfile::tempdir().unwrap();
        let test_dir_path = test_bed.path();
        let file_to_exclude = test_dir_path.join("subdir/file_to_exclude.txt");

        fs::create_dir_all(file_to_exclude.parent().unwrap()).unwrap();
        fs::File::create(&file_to_exclude).unwrap();

        let rules = gen_rules(
            "
            subdir/file_to_exclude.txt
        ",
        );

        let scoped = {
            let mut s = HashMap::new();
            s.insert(test_dir_path.to_path_buf(), rules);
            s
        };

        let absolute = vec![];

        let gitignore = GitIgnore::new_for_testing(scoped, absolute);

        assert!(check_gitignore(&gitignore, file_to_exclude));
    }

    #[test]
    fn test_check_rules_exclude_multi_levels_scoped_negate() {
        let test_bed = tempfile::tempdir().unwrap();
        let test_dir_path = test_bed.path();
        let subdir = test_dir_path.join("subdir");
        let file_to_include = test_dir_path.join("subdir/file_to_include.txt");

        fs::create_dir_all(&subdir).unwrap();
        fs::File::create(&file_to_include).unwrap();

        // The root .gitignore ignores all .txt file, which should include subdir/file_to_include.txt..
        let root_rules = gen_rules(
            "
            **/*.txt
        ",
        );

        // .. but it is ovcerridden by the .gitignore in subdir, which explicitly includes file_to_include.txt
        let subdir_rules = gen_rules(
            "
          !file_to_include.txt
        ",
        );

        let scoped = {
            let mut s = HashMap::new();
            s.insert(test_dir_path.to_path_buf(), root_rules);
            s.insert(subdir, subdir_rules);
            s
        };

        let absolute = vec![];

        let gitignore = GitIgnore::new_for_testing(scoped, absolute);

        assert!(!check_gitignore(&gitignore, file_to_include));
    }
}
