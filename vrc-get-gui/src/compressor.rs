use crate::commands::AsyncCommandContext;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use zip::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

#[derive(Debug, Clone, Serialize, specta::Type)]
pub struct TauriCreateBackupProgress {
    total: usize,
    proceed: usize,
    last_proceed: String,
}

#[derive(Debug)]
pub enum CompressError {
    Io(std::io::Error),
    Zip(zip::result::ZipError),
    TaskJoin(tokio::task::JoinError),
    Cancelled,
}

impl From<std::io::Error> for CompressError {
    fn from(value: std::io::Error) -> Self {
        CompressError::Io(value)
    }
}

impl From<zip::result::ZipError> for CompressError {
    fn from(value: zip::result::ZipError) -> Self {
        CompressError::Zip(value)
    }
}

struct SyncSemaphore {
    pair: Arc<(std::sync::Mutex<usize>, std::sync::Condvar)>,
    max: usize,
}

impl SyncSemaphore {
    fn new(max: usize) -> Self {
        Self {
            pair: Arc::new((std::sync::Mutex::new(0), std::sync::Condvar::new())),
            max,
        }
    }

    fn acquire(&self) -> SyncSemaphoreGuard {
        let (lock, cvar) = &*self.pair;
        let mut count = lock.lock().unwrap();
        while *count >= self.max {
            count = cvar.wait(count).unwrap();
        }
        *count += 1;
        SyncSemaphoreGuard {
            pair: self.pair.clone(),
        }
    }
}

struct SyncSemaphoreGuard {
    pair: Arc<(std::sync::Mutex<usize>, std::sync::Condvar)>,
}

impl Drop for SyncSemaphoreGuard {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.pair;
        let mut count = lock.lock().unwrap();
        *count -= 1;
        cvar.notify_one();
    }
}

pub(crate) enum CompressEntry {
    Dir {
        relative_path: String,
    },
    File {
        relative_path: String,
        absolute_path: PathBuf,
    },
}

struct WriteState {
    zip: Option<ZipWriter<std::fs::File>>,
    next_write_idx: usize,
    pending: BTreeMap<usize, (String, Vec<u8>)>,
    total: usize,
    ctx: AsyncCommandContext<TauriCreateBackupProgress>,
}

impl WriteState {
    fn new(
        zip: ZipWriter<std::fs::File>,
        total: usize,
        ctx: AsyncCommandContext<TauriCreateBackupProgress>,
    ) -> Self {
        Self {
            zip: Some(zip),
            next_write_idx: 0,
            pending: BTreeMap::new(),
            total,
            ctx,
        }
    }

    fn submit(
        &mut self,
        idx: usize,
        display_name: String,
        data: Vec<u8>,
    ) -> Result<(), CompressError> {
        self.pending.insert(idx, (display_name, data));
        while let Some((name, data)) = self.pending.remove(&self.next_write_idx) {
            if let Some(zip) = self.zip.as_mut() {
                let mut cur = Cursor::new(data);
                let mut archive = ZipArchive::new(&mut cur)?;
                let zipfile = archive.by_index_raw(0)?;
                zip.raw_copy_file(zipfile)?;
            }
            self.next_write_idx += 1;
            let _ = self.ctx.emit(TauriCreateBackupProgress {
                total: self.total,
                proceed: self.next_write_idx,
                last_proceed: name,
            });
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<(), CompressError> {
        if let Some(zip) = self.zip.take() {
            let file = zip.finish()?;
            file.sync_all()?;
        }
        Ok(())
    }
}

fn file_options(method: CompressionMethod, compression_level: Option<i64>) -> SimpleFileOptions {
    let mut opts = SimpleFileOptions::default().compression_method(method);
    if method == CompressionMethod::Deflated {
        opts = opts.compression_level(compression_level);
    } else {
        opts = opts.compression_level(None);
    }
    opts
}

fn entry_to_partial_zip(
    entry: &CompressEntry,
    method: CompressionMethod,
    compression_level: Option<i64>,
) -> Result<Vec<u8>, CompressError> {
    let mut cur = Cursor::new(Vec::new());
    {
        let mut partial_zip = ZipWriter::new(&mut cur);
        match entry {
            CompressEntry::Dir { relative_path } => {
                let opts = file_options(CompressionMethod::Stored, None);
                partial_zip.start_file(relative_path, opts)?;
                partial_zip.write_all(&[])?;
            }
            CompressEntry::File {
                relative_path,
                absolute_path,
            } => {
                let opts = file_options(method, compression_level);
                let data = std::fs::read(absolute_path).map_err(CompressError::Io)?;
                partial_zip.start_file(relative_path, opts)?;
                partial_zip.write_all(&data)?;
            }
        }
        partial_zip.finish()?;
    }
    Ok(cur.into_inner())
}

pub(crate) async fn parallel_compress_zip(
    entries: Vec<CompressEntry>,
    destination: PathBuf,
    compression_method: CompressionMethod,
    compression_level: Option<i64>,
    ctx: AsyncCommandContext<TauriCreateBackupProgress>,
    token: CancellationToken,
) -> Result<(), CompressError> {
    let total = entries.len();

    let _ = ctx.emit(TauriCreateBackupProgress {
        total,
        proceed: 0,
        last_proceed: "Collecting files".to_string(),
    });

    if entries.is_empty() {
        tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&destination)?;
            let zip = ZipWriter::new(file);
            let file = zip.finish()?;
            file.sync_all()?;
            Ok::<(), CompressError>(())
        })
        .await
        .map_err(CompressError::TaskJoin)??;
        return Ok(());
    }

    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&destination)
            .map_err(CompressError::Io)?;
        let write_state = Arc::new(Mutex::new(WriteState::new(
            ZipWriter::new(file),
            total,
            ctx,
        )));

        let parallelism = std::thread::available_parallelism().map_or(1, |n| n.get());
        let semaphore = SyncSemaphore::new(parallelism);

        entries.par_iter().enumerate().try_for_each(
            |(idx, entry)| -> Result<(), CompressError> {
                if token.is_cancelled() {
                    return Err(CompressError::Cancelled);
                }

                let display_name = match entry {
                    CompressEntry::Dir { relative_path } => relative_path.clone(),
                    CompressEntry::File { relative_path, .. } => relative_path.clone(),
                };

                let _guard = semaphore.acquire();

                let data = entry_to_partial_zip(entry, compression_method, compression_level)?;

                write_state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .submit(idx, display_name, data)?;

                Ok(())
            },
        )?;

        write_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .finish()
    })
    .await
    .map_err(CompressError::TaskJoin)??;

    Ok(())
}
