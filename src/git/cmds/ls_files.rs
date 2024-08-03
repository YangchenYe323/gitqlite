
use chrono::DateTime;

use crate::{
    cli::LsFilesArgs,
    git::{
        model::{Index, ModeType},
        utils::{get_gitqlite_connection},
    },
};

pub fn do_ls_files(arg: LsFilesArgs) -> crate::Result<()> {
    let conn = get_gitqlite_connection()?;

    let index = Index::read_from_conn(&conn)?;

    for entry in index.entries {
        println!("{}", entry.name);
        if arg.verbose {
            let file_type = match entry.mode_type {
                ModeType::Regular => "Regular File",
                ModeType::Symlink => "Symlink",
                ModeType::Gitlink => "Gitlink",
            };
            println!("    [{}] with perm {:o} ", file_type, entry.mode_perms);
            println!("    on blob: {}", entry.sha);

            let ctime = DateTime::from_timestamp_nanos(entry.ctime);
            let mtime = DateTime::from_timestamp_nanos(entry.mtime);
            println!(
                "    created on {}, last modified on {}",
                ctime.format("%Y-%m-%d %H:%M:%S.%f"),
                mtime.format("%Y-%m-%d %H:%M:%S.%f")
            );

            println!("    device {}, inode {}", entry.dev, entry.ino);

            println!("    user {}, group {}", entry.uid, entry.gid);

            println!(
                "    flags: stage={}, assume_valid={}",
                entry.flag_stage, entry.flag_assume_valid
            );
        }
    }

    Ok(())
}
