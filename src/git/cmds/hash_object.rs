use anyhow::anyhow;
use sha1::Digest;

use std::{fs, io::Read, path::Path};

use crate::{
    cli::{HashObjectArgs, ObjectType},
    git::{
        model::{Blob, Hashable, Sha1Id},
        utils::get_gitqlite_connection,
    },
};

pub fn do_hash_object(arg: HashObjectArgs) -> crate::Result<()> {
    let HashObjectArgs { type_, write, file } = arg;
    let conn = get_gitqlite_connection()?;

    match type_ {
        ObjectType::Blob => {
            let blob = construct_blob_from_file(&file)?;
            if write {
                blob.persist(&conn)?;
            }
            println!("ID for {}: {}", file.display(), blob.blob_id);
        }
        _ => unimplemented!(),
    }

    Ok(())
}

fn construct_blob_from_file(path: impl AsRef<Path>) -> crate::Result<Blob<Sha1Id>> {
    let path = path.as_ref();

    if !path.is_file() {
        return Err(anyhow!(
            "Could not hash a non-file path to a blob: {}",
            path.display()
        ));
    }

    let data = {
        let mut f = fs::File::open(path)?;
        let mut buffer = Vec::with_capacity(1024);
        f.read_to_end(&mut buffer)?;
        buffer
    };

    let blob = Blob::new(data);

    let blob_id = blob.hash(sha1::Sha1::new());

    Ok(blob.with_id(blob_id))
}
