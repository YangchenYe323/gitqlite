pub mod cli;
pub mod git;
mod repo;

pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;