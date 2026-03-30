use crate::commands::AsyncCommandContext;
use crate::utils::FileSystemTree;
use async_zip::base::write::ZipFileWriter;
use async_zip::{Compression, DeflateOption, ZipEntryBuilder};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::fs::File;
use tokio::sync::Semaphore;
use tokio_util::compat::Compat;

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TauriCreateBackupProgress {
    total: usize,
    proceed: usize,
    last_proceed: String,
}

#[derive(Debug)]
pub enum CompressError {
    Io(std::io::Error),
    Zip(async_zip::error::ZipError),
    TaskJoin(tokio::task::JoinError),
    Semaphore(tokio::sync::AcquireError),
}

impl From<std::io::Error> for CompressError {
    fn from(value: std::io::Error) -> Self {
        CompressError::Io(value)
    }
}

impl From<async_zip::error::ZipError> for CompressError {
    fn from(value: async_zip::error::ZipError) -> Self {
        CompressError::Zip(value)
    }
}

impl From<tokio::task::JoinError> for CompressError {
    fn from(value: tokio::task::JoinError) -> Self {
        CompressError::TaskJoin(value)
    }
}

impl From<tokio::sync::AcquireError> for CompressError {
    fn from(value: tokio::sync::AcquireError) -> Self {
        CompressError::Semaphore(value)
    }
}

struct CompressedData {
    bytes: Vec<u8>,
    crc32: u32,
    uncompressed_size: u64,
}

struct WriteMessage {
    index: usize,
    relative_path: String,
    data: Option<CompressedData>,
}

impl WriteMessage {
    fn new(index: usize, relative_path: String, data: Option<CompressedData>) -> Self {
        Self {
            index,
            relative_path,
            data,
        }
    }
}

struct WriteState {
    zip: Option<ZipFileWriter<Compat<File>>>,
    compression: Compression,
    deflate_option: DeflateOption,
    next_write_idx: usize,
    pending: BTreeMap<usize, (String, Option<CompressedData>)>,

    rx: std::sync::mpsc::Receiver<WriteMessage>,
}

impl WriteState {
    fn new(
        zip: ZipFileWriter<Compat<File>>,
        compression: Compression,
        deflate_option: DeflateOption,
        rx: std::sync::mpsc::Receiver<WriteMessage>,
    ) -> Self {
        Self {
            zip: Some(zip),
            compression,
            deflate_option,
            next_write_idx: 0,
            pending: BTreeMap::new(),
            rx,
        }
    }

    async fn run(mut self) -> Result<(), CompressError> {
        while let Ok(msg) = self.rx.recv() {
            self.submit(msg.index, msg.relative_path, msg.data).await?;
        }
        self.finish().await
    }

    async fn submit(
        &mut self,
        idx: usize,
        relative_path: String,
        data: Option<CompressedData>,
    ) -> Result<(), CompressError> {
        self.pending.insert(idx, (relative_path, data));

        while let Some((name, entry_data)) = self.pending.remove(&self.next_write_idx) {
            if let Some(zip) = self.zip.as_mut() {
                match entry_data {
                    None => {
                        let entry = ZipEntryBuilder::new(name.into(), self.compression.clone())
                            .deflate_option(self.deflate_option.clone());
                        zip.write_entry_whole(entry.build(), b"").await?;
                    }
                    Some(cd) => {
                        let entry = ZipEntryBuilder::new(name.into(), self.compression.clone())
                            .deflate_option(self.deflate_option.clone())
                            .crc32(cd.crc32)
                            .uncompressed_size(cd.uncompressed_size);
                        zip.write_entry_whole_precompressed(entry.build(), &cd.bytes)
                            .await?;
                    }
                }
            }
            self.next_write_idx += 1;
        }
        Ok(())
    }

    async fn finish(&mut self) -> Result<(), CompressError> {
        if let Some(zip) = self.zip.take() {
            zip.close().await?;
        }
        Ok(())
    }
}

pub(crate) async fn parallel_compress_zip(
    tree: FileSystemTree,
    destination: PathBuf,
    compression: Compression,
    deflate_option: DeflateOption,
    ctx: AsyncCommandContext<TauriCreateBackupProgress>,
) -> Result<(), CompressError> {
    let total = tree.count_all();

    let _ = ctx.emit(TauriCreateBackupProgress {
        total,
        proceed: 0,
        last_proceed: "Collecting files".to_string(),
    });

    let file = File::create_new(&destination).await?;
    let writer = ZipFileWriter::with_tokio(file);

    let (sender, rx) = std::sync::mpsc::channel();
    let write_state = WriteState::new(writer, compression.clone(), deflate_option.clone(), rx);

    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let semaphore = Arc::new(Semaphore::new(threads));
    let proceed = Arc::new(AtomicUsize::new(0));

    let merge_task = tokio::spawn(write_state.run());

    let mut handles = vec![];

    for (idx, entry) in tree.recursive().enumerate() {
        if entry.is_dir() {
            let relative_path = entry.relative_path().to_string();
            let _ = sender.send(WriteMessage::new(idx, relative_path.clone(), None));
            let p = proceed.fetch_add(1, Ordering::Relaxed) + 1;
            let _ = ctx.emit(TauriCreateBackupProgress {
                total,
                proceed: p,
                last_proceed: relative_path,
            });
        } else {
            let permit = semaphore.clone().acquire_owned().await?;

            let relative_path = entry.relative_path().to_string();
            let absolute_path = entry.absolute_path().to_path_buf();

            let sender = sender.clone();
            let compression = compression.clone();
            let deflate_option = deflate_option.clone();
            let ctx = ctx.clone();
            let proceed = proceed.clone();

            let handle: tokio::task::JoinHandle<Result<(), CompressError>> =
                tokio::task::spawn(async move {
                    let cd = {
                        let raw_data = tokio::fs::read(&absolute_path).await?;
                        let crc32 = async_zip::base::write::crc32(&raw_data);
                        let uncompressed_size = raw_data.len() as u64;

                        let bytes = async_zip::base::write::compress(
                            &ZipEntryBuilder::new(relative_path.clone().into(), compression)
                                .deflate_option(deflate_option)
                                .build(),
                            &raw_data,
                        )
                        .await;

                        CompressedData {
                            bytes,
                            crc32,
                            uncompressed_size,
                        }
                    };

                    let _ = sender.send(WriteMessage::new(idx, relative_path.clone(), Some(cd)));
                    let p = proceed.fetch_add(1, Ordering::Relaxed) + 1;
                    let _ = ctx.emit(TauriCreateBackupProgress {
                        total,
                        proceed: p,
                        last_proceed: relative_path,
                    });

                    drop(permit);

                    Ok(())
                });

            handles.push(handle);
        }
    }

    drop(sender);

    for handle in handles {
        handle.await??;
    }

    merge_task.await??;

    Ok(())
}
