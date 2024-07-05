//! This module implements the gitqlite index file, which implements the staging area.
//! Gitqlite uses JSON as the index format as compared to a custom binary format used by git,
//! but the content is roughly on par

use serde::{Deserialize, Serialize};

use super::model::Sha1Id;

#[derive(Debug, Serialize, Deserialize)]
pub enum ModeType {
    Regular,
    Symlink,
    Gitlink,
}

/// [`Index`] represents the whole staging area
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Index {
    pub version: u64,
    pub entries: Vec<IndexEntry>,
}

/// [`IndexEntry`] represents one entry in the staging area, which is the snapshot of a file
/// in a point in time
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexEntry {
    /// The last time the file's metadata has changed, in nanosecond
    pub ctime: i64,
    /// The last time the file's data has changed, in nanosecond
    pub mtime: i64,
    /// The ID of device containing this file
    pub dev: u64,
    /// The inode number of the file
    pub ino: u64,
    /// Mode type
    pub mode_type: ModeType,
    /// Mode permissions
    pub mode_perms: u32,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
    /// Size of the file in bytes
    pub fsize: usize,
    /// SHA of the object
    pub sha: Sha1Id,
    /// TODO: fill doc
    pub flag_assume_valid: bool,
    /// TODO: fill doc
    pub flag_stage: u8,
    /// Full path of the object
    pub name: String,
}
