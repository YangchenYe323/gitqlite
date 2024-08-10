use serde::{Deserialize, Serialize};

/// [`FileType`] represents a file type on the file system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Regular,
    Symlink,
}

impl FileType {
    pub fn num(self) -> u32 {
        match self {
            FileType::Regular => 0o100,
            FileType::Symlink => 0o120,
        }
    }
}
