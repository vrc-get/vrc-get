//! This file contains tests for the virtual file system.

use futures::AsyncReadExt;
use vrc_get_vpm::io::IoTrait;

mod common;

#[test]
fn test() {
    futures::executor::block_on(async {
        let vfs = common::VirtualFileSystem::new();
        let content = b"";
        vfs.add_file("test".as_ref(), content).await.unwrap();

        let mut read = Vec::new();
        vfs.open("test".as_ref())
            .await
            .unwrap()
            .read_to_end(&mut read)
            .await
            .unwrap();

        assert_eq!(read, content);
    });
}
