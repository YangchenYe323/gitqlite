use clap::Parser;
use gitqlite::git;
use gitqlite::cli;

use git::cmds::add::do_add;
use git::cmds::cat_file::do_cat_file;
use git::cmds::check_ignore::do_check_ignore;
use git::cmds::commit::do_commit;
use git::cmds::config::do_config;
use git::cmds::hash_object::do_hash_object;
use git::cmds::init::do_init;
use git::cmds::ls_files::do_ls_files;
use git::cmds::rm::do_rm;
use git::cmds::status::do_status;

fn main() -> gitqlite::Result<()> {
    let cli = cli::GitCli::parse();

    match cli.command {
        cli::GitCommand::Init(arg) => do_init(arg),
        cli::GitCommand::CatFile(arg) => do_cat_file(arg),
        cli::GitCommand::HashObject(arg) => do_hash_object(arg),
        cli::GitCommand::LsFiles(arg) => do_ls_files(arg),
        cli::GitCommand::CheckIgnore(arg) => do_check_ignore(arg),
        cli::GitCommand::Config(arg) => do_config(arg),
        cli::GitCommand::Status(arg) => do_status(arg),
        cli::GitCommand::Rm(arg) => do_rm(arg),
        cli::GitCommand::Add(arg) => do_add(arg),
        cli::GitCommand::Commit(arg) => do_commit(arg),
    }
}
