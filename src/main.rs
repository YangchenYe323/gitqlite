use clap::Parser;
use git::do_init;

mod cli;
mod git;

pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let cli = cli::GitCli::parse();


    match cli.command {
        cli::GitCommand::Init(arg) => {
            do_init(arg)?;
        }
    }

    Ok(())
}
