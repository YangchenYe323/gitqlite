//! This module provides utility classes for getting file metadata in a cross-platform way

use std::fs;

#[cfg(target_os = "unix")]
use std::os::unix::fs::MetadataExt;
#[cfg(target_os = "windows")]
use std::os::windows::fs::MetadataExt;

/// Provides a gitqlite specific trait to fetch necessary metadata from the file cross-platform
pub trait GitqliteFileMetadataExt {
    fn g_ctime(&self) -> i64;
    fn g_mtime(&self) -> i64;
    fn g_dev(&self) -> u64;
    fn g_ino(&self) -> u64;
    fn g_mode_perms(&self) -> u32;
    fn g_uid(&self) -> u32;
    fn g_gid(&self) -> u32;
    fn g_fsize(&self) -> u64;
}

#[cfg(target_os = "windows")]
impl GitqliteFileMetadataExt for fs::Metadata {
    fn g_ctime(&self) -> i64 {
        self.creation_time() as i64 * 10
    }

    fn g_mtime(&self) -> i64 {
        self.last_write_time() as i64 * 10
    }

    fn g_dev(&self) -> u64 {
        0
    }

    fn g_ino(&self) -> u64 {
        0
    }

    fn g_mode_perms(&self) -> u32 {
        0
    }

    fn g_uid(&self) -> u32 {
        0
    }

    fn g_gid(&self) -> u32 {
        0
    }

    fn g_fsize(&self) -> u64 {
        self.file_size()
    }
}

#[cfg(target_os = "unix")]
impl GitqliteFileMetadataExt for fs::Metadata {
    fn g_ctime(&self) -> i64 {
        self.ctime_nsec()
    }

    fn g_mtime(&self) -> i64 {
        self.mtime_nsec()
    }

    fn g_dev(&self) -> u64 {
        self.dev()
    }

    fn g_ino(&self) -> u64 {
        self.ino()
    }

    fn g_mode_perms(&self) -> u32 {
        self.mode()
    }

    fn g_uid(&self) -> u32 {
        self.uid()
    }

    fn g_gid(&self) -> u32 {
        self.gid()
    }

    fn g_fsize(&self) -> u64 {
        self.size()
    }
}
