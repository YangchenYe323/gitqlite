use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[clap(version, about, long_about = None)]
pub struct GitCli {
    #[command(subcommand)]
    pub command: GitCommand,
}

#[derive(Subcommand, Clone)]
pub enum GitCommand {
    /// Create an empty Git repository or reinitialize an existing one
    Init(InitArgs),

    /// Provide contents or details of repository objects
    CatFile(CatFileArgs),

    /// Compute object ID and optionally create an object from a file
    HashObject(HashObjectArgs),

    /// Show content of the staging area
    LsFiles(LsFilesArgs),

    /// Check whether the file is excluded by .gitignore (or other input files to the exclude mechanism) and output the path if it is excluded.
    CheckIgnore(CheckIgnoreArgs),
}

#[derive(Args, Clone)]
pub struct InitArgs {
    /// Set the initial branch name of the new repository
    #[arg(long, short = 'b')]
    initial_branch: Option<String>,
}

#[derive(Args, Clone)]
pub struct CatFileArgs {
    /// The type of the requested object
    pub type_: ObjectType,
    /// The name of the object to show
    pub object: String,
}

#[derive(ValueEnum, Clone)]
pub enum ObjectType {
    Blob,
    Tree,
    Commit,
}

#[derive(Args, Clone)]
pub struct HashObjectArgs {
    /// Specify the type of object to be created.
    #[arg(short = 't', default_value = "blob")]
    pub type_: ObjectType,

    /// Actually write the object into the object database.
    #[arg(short = 'w')]
    pub write: bool,

    /// Path of local file/directory to create an object for
    pub file: PathBuf,
}

#[derive(Args, Clone)]
pub struct LsFilesArgs {
    /// Show verbose output of the staged files
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(Args, Clone)]
pub struct CheckIgnoreArgs {
    /// The pathname to check whether the path is excluded by gitqlite
    pub path: PathBuf,
}
