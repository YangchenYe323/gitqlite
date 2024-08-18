pub mod cli;
pub mod git;
pub mod repo;

pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;