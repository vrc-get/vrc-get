use crate::io::{IoTrait, SymlinkKind};
use futures::io;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use tokio_util::compat::TokioAsyncReadCompatExt;

pub(crate) async fn copy_recursive(
    src_dir: Box<Path>,
    dst_io: &impl IoTrait,
    dst_dir: PathBuf,
) -> io::Result<()> {
    // TODO: parallelize & speedup
    let mut queue = VecDeque::new();
    queue.push_front((src_dir.into_path_buf(), dst_dir));

    while let Some((src_dir, dst_dir)) = queue.pop_back() {
        let mut iter = tokio::fs::read_dir(src_dir).await?;
        dst_io.create_dir_all(&dst_dir).await?;
        while let Some(entry) = iter.next_entry().await? {
            let file_type = entry.file_type().await?;
            let src = entry.path();
            let dst = dst_dir.join(entry.file_name());

            if file_type.is_symlink() {
                // symlink: just copy
                let symlink = tokio::fs::read_link(src).await?;
                if symlink.is_absolute() {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "absolute symlink detected",
                    ));
                }

                #[cfg(not(windows))]
                let kind: Option<SymlinkKind> = None;
                #[cfg(windows)]
                let kind = {
                    use std::os::windows::fs::FileTypeExt;
                    if file_type.is_symlink_file() {
                        Some(SymlinkKind::File)
                    } else if file_type.is_symlink_dir() {
                        Some(SymlinkKind::Directory)
                    } else {
                        None
                    }
                };

                dst_io.symlink(dst, kind, symlink).await?;
            } else if file_type.is_file() {
                let mut src_file = tokio::fs::File::open(src).await?.compat();
                let mut dst_file = dst_io.create_new(dst).await?;
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
