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
    /// Show the working tree status
    Status(StatusArgs),
    /// Get and set repository or global options
    Config(ConfigArgs),
    /// Remove files from the working tree and from the index
    Rm(RmArgs),
    /// Add file contents to the index
    Add(AddArgs),
    /// Record changes to the repository
    Commit(CommitArgs),
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

#[derive(Args, Clone)]
pub struct StatusArgs {}

#[derive(Args, Clone)]
pub struct ConfigArgs {
    /// config entry name (e.g., user.email)
    pub name: String,

    /// config entry value (get config entry if none)
    pub value: Option<String>,

    /// show origin of config (file)
    #[arg(long)]
    pub show_origin: bool,

    /// use system config file
    #[arg(long)]
    pub system: bool,

    /// use global config file
    #[arg(long)]
    pub global: bool,

    /// use repository config file
    #[arg(long)]
    pub local: bool,
}

#[derive(Args, Clone)]
pub struct RmArgs {
    /// File to remove (recursively removing directory is not supported yet)
    pub path: PathBuf,
    /// Use this option to unstage and remove paths only from the index. Working tree files, whether modified or not, will be left alone.
    #[arg(long)]
    pub cached: bool,
}

#[derive(Args, Clone)]
pub struct AddArgs {
    /// File to add (recursively adding directory is not supported yet)
    pub path: PathBuf,
}

/// Record changes to the repository
#[derive(Args, Clone)]
pub struct CommitArgs {
    /// Use the given message as the commit message.
    #[arg(long, short)]
    pub message: String,
}
