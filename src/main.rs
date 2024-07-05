use clap::Parser;
use git::{do_cat_file, do_hash_object, do_init, do_ls_files};

mod cli;
mod git;

pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let cli = cli::GitCli::parse();

    match cli.command {
        cli::GitCommand::Init(arg) => do_init(arg),
        cli::GitCommand::CatFile(arg) => do_cat_file(arg),
        cli::GitCommand::HashObject(arg) => do_hash_object(arg),
        cli::GitCommand::LsFiles(arg) => do_ls_files(arg),
        // _ => unimplemented!(),
    }
}
