use std::collections::VecDeque;
use std::io;
use std::path::Path;
use tokio::fs::create_dir_all;

pub(crate) async fn copy_recursive(src_dir: Box<Path>, dst_dir: Box<Path>) -> io::Result<()> {
    // TODO: parallelize & speedup
    let mut queue = VecDeque::new();
    queue.push_front((src_dir.into_path_buf(), dst_dir.into_path_buf()));

    while let Some((src_dir, dst_dir)) = queue.pop_back() {
        let mut iter = tokio::fs::read_dir(src_dir).await?;
        create_dir_all(&dst_dir).await?;
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

                #[cfg(unix)]
                tokio::fs::symlink(dst, symlink).await?;
                #[cfg(windows)]
                {
                    use std::os::windows::fs::FileTypeExt;
                    if file_type.is_symlink_file() {
                        tokio::fs::symlink_file(dst, symlink).await?;
                    } else {
                        assert!(file_type.is_symlink_dir(), "unknown symlink");
                        tokio::fs::symlink_dir(dst, symlink).await?;
                    }
                }
                #[cfg(not(any(unix, windows)))]
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "platform without symlink detected",
                ));
            } else if file_type.is_file() {
                tokio::fs::copy(src, dst).await?;
            } else if file_type.is_dir() {
                //copy_recursive(&src, &dst).await?;
                queue.push_front((src, dst));
            } else {
                panic!("unknown file type: none of file, dir, symlink")
            }
        }
    }

    Ok(())
}
