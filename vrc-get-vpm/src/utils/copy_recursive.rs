use crate::io;
use crate::io::{DirEntry, IoTrait};
use futures::prelude::*;
use std::collections::VecDeque;
use std::path::PathBuf;

pub(crate) async fn copy_recursive(
    src_io: &impl IoTrait,
    src_dir: PathBuf,
    dst_io: &impl IoTrait,
    dst_dir: PathBuf,
) -> io::Result<()> {
    // TODO: parallelize & speedup
    let mut queue = VecDeque::new();
    queue.push_front((src_dir, dst_dir));

    while let Some((src_dir, dst_dir)) = queue.pop_back() {
        let mut iter = src_io.read_dir(&src_dir).await?;
        dst_io.create_dir_all(&dst_dir).await?;
        while let Some(entry) = iter.try_next().await? {
            let file_type = entry.file_type().await?;
            let src = src_dir.join(entry.file_name());
            let dst = dst_dir.join(entry.file_name());

            if file_type.is_file() {
                let mut src_file = src_io.open(src.as_ref()).await?;
                let mut dst_file = dst_io.create_new(dst.as_ref()).await?;
                io::copy(&mut src_file, &mut dst_file).await?;
            } else if file_type.is_dir() {
                //copy_recursive(&src, dst_io, &dst).await?;
                queue.push_front((src, dst));
            } else {
                panic!("unknown file type: none of file, dir, symlink")
            }
        }
    }

    Ok(())
}
