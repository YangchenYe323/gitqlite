use clap::{Args, Parser, Subcommand};

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
}

#[derive(Args, Clone)]
pub struct InitArgs {
    /// Set the initial branch name of the new repository
    #[arg(long, short = 'b')]
    initial_branch: Option<String>,
}
