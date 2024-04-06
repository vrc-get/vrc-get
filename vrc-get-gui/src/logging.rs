use arc_swap::ArcSwapOption;
use log::{Log, Metadata, Record};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use serde::Serialize;
use std::cmp::Reverse;
use std::fmt::{Display, Formatter};
use std::io::Write as _;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Manager};
use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo};

static APP_HANDLE: ArcSwapOption<AppHandle> = ArcSwapOption::const_empty();

pub fn set_app_handle(handle: AppHandle) {
    APP_HANDLE.store(Some(Arc::new(handle)));
}

pub fn initialize_logger() -> DefaultEnvironmentIo {
    let (sender, receiver) = mpsc::channel::<LogChannelMessage>();
    let logger = Logger { sender };

    log::set_max_level(log::LevelFilter::Debug);
    log::set_boxed_logger(Box::new(logger)).expect("error while setting logger");

    let io = DefaultEnvironmentIo::new_default();

    start_logging_thread(receiver, &io);

    io
}

fn start_logging_thread(receiver: mpsc::Receiver<LogChannelMessage>, io: &DefaultEnvironmentIo) {
    let log_folder = io.resolve("vrc-get-logs".as_ref());
    std::fs::create_dir_all(&log_folder).ok();
    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%d_%H-%M-%S.%6f")
        .to_string();
    let log_file = log_folder.join(format!("vrc-get-{}.log", timestamp));

    let log_file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(file) => {
            log::info!("logging to file {}", log_file.display());
            Some(file)
        }
        Err(e) => {
            log::error!("error while opening log file: {}", e);
            None
        }
    };

    std::thread::Builder::new()
        .name("logging".to_string())
        .spawn(move || {
            logging_thread_main(receiver, log_file);
        })
        .expect("error while starting logging thread");

    std::thread::Builder::new()
        .name("remove-old-logs".to_string())
        .spawn(move || remove_old_logs(log_folder))
        .expect("error while starting remove-old-logs thread");
}

fn is_log_file_name(name: &str) -> bool {
    // vrc-get-yyyy-mm-dd_hh-mm-ss.ssssss.log
    if name.len() != "vrc-get-yyyy-mm-dd_hh-mm-ss.ssssss.log".len() {
        return false;
    }
    let Some(name) = name.strip_prefix("vrc-get-") else {
        return false;
    };
    let Some(name) = name.strip_suffix(".log") else {
        return false;
    };

    //              00000000001111111111222222
    //              01234567890123456789012345
    // now, name is yyyy-mm-dd_hh-mm-ss.ssssss
    let name = name.as_bytes();
    let Ok(name) = <&[u8; 26]>::try_from(name) else {
        return false;
    };

    if name[4] != b'-'
        || name[7] != b'-'
        || name[10] != b'_'
        || name[13] != b'-'
        || name[16] != b'-'
        || name[19] != b'.'
    {
        return false;
    }

    name[0..4].iter().all(u8::is_ascii_digit)
        && name[5..7].iter().all(u8::is_ascii_digit)
        && name[8..10].iter().all(u8::is_ascii_digit)
        && name[11..13].iter().all(u8::is_ascii_digit)
        && name[14..16].iter().all(u8::is_ascii_digit)
        && name[17..19].iter().all(u8::is_ascii_digit)
        && name[20..26].iter().all(u8::is_ascii_digit)
}

fn remove_old_logs(log_folder: std::path::PathBuf) {
    let read_dir = match std::fs::read_dir(&log_folder) {
        Ok(read_dir) => read_dir,
        Err(e) => {
            log::error!("error while reading log folder: {}", e);
            return;
        }
    };

    let entries = match read_dir.collect::<Result<Vec<_>, _>>() {
        Ok(entries) => entries,
        Err(e) => {
            log::error!("error while reading log folder: {}", e);
            return;
        }
    };

    let mut log_files = entries
        .into_iter()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            if is_log_file_name(&name) {
                Some((name, entry))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    log_files.sort_by_key(|(name, _)| Reverse(name.clone()));

    static MAX_LOGS: usize = 30;

    for (name, _) in log_files.iter().take(MAX_LOGS) {
        log::debug!("log to keep: {}", name);
    }

    for (name, _) in log_files.iter().skip(MAX_LOGS) {
        match std::fs::remove_file(log_folder.join(name)) {
            Ok(()) => log::debug!("removed old log: {}", name),
            Err(e) => log::debug!("error while removing old log: {}: {}", name, e),
        }
    }
}

fn logging_thread_main(
    receiver: mpsc::Receiver<LogChannelMessage>,
    mut log_file: Option<std::fs::File>,
) {
    for message in receiver {
        match message {
            LogChannelMessage::Log(entry) => {
                let message = format!("{}", entry);
                // log to console
                eprintln!("{}", message);

                // log to file
                if let Some(log_file) = log_file.as_mut() {
                    log_err(writeln!(log_file, "{}", message));
                }

                // add to buffer
                {
                    let mut buffer = LOG_BUFFER.lock().unwrap();
                    buffer.push(entry.clone());
                }

                // send to tauri
                if let Some(app_handle) = APP_HANDLE.load().as_ref() {
                    app_handle
                        .emit_all("log", Some(entry))
                        .expect("error while emitting log event");
                }
            }
            LogChannelMessage::Flush(sync) => {
                if let Some(log_file) = log_file.as_mut() {
                    log_err(log_file.flush());
                    sync.send(()).ok();
                }
            }
        }
    }
}

enum LogChannelMessage {
    Log(LogEntry),
    Flush(mpsc::Sender<()>),
}

pub(crate) fn get_log_entries() -> Vec<LogEntry> {
    LOG_BUFFER.lock().unwrap().to_vec()
}

#[derive(Serialize, specta::Type, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => "ERROR".fmt(f),
            LogLevel::Warn => "WARN".fmt(f),
            LogLevel::Info => "INFO".fmt(f),
            LogLevel::Debug => "DEBUG".fmt(f),
            LogLevel::Trace => "TRACE".fmt(f),
        }
    }
}

impl From<log::Level> for LogLevel {
    fn from(value: log::Level) -> Self {
        match value {
            log::Level::Error => LogLevel::Error,
            log::Level::Warn => LogLevel::Warn,
            log::Level::Info => LogLevel::Info,
            log::Level::Debug => LogLevel::Debug,
            log::Level::Trace => LogLevel::Trace,
        }
    }
}

#[derive(Serialize, specta::Type, Clone)]
pub(crate) struct LogEntry {
    #[serde(serialize_with = "to_rfc3339_micros")]
    time: chrono::DateTime<chrono::Local>,
    level: LogLevel,
    target: String,
    message: String,
}

fn to_rfc3339_micros<S>(
    time: &chrono::DateTime<chrono::Local>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    time.to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
        .serialize(serializer)
}

impl LogEntry {
    pub fn new(record: &Record) -> Self {
        LogEntry {
            time: chrono::Local::now(),
            level: record.level().into(),
            target: record.target().to_string(),
            message: format!("{}", record.args()),
        }
    }
}

impl Display for LogEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} [{: >5}] {}: {}",
            self.time
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, false),
            self.level,
            self.target,
            self.message
        )
    }
}

static LOG_BUFFER: Mutex<ConstGenericRingBuffer<LogEntry, 256>> =
    Mutex::new(ConstGenericRingBuffer::new());

struct Logger {
    sender: mpsc::Sender<LogChannelMessage>,
}

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // TODO: configurable
        metadata.level() <= log::Level::Info
            || metadata.target().starts_with("vrc_get") && metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let entry = LogEntry::new(record);
        self.sender.send(LogChannelMessage::Log(entry)).ok();
    }

    fn flush(&self) {
        let (sync, receiver) = mpsc::channel();
        self.sender.send(LogChannelMessage::Flush(sync)).ok();
        receiver.recv().ok();
    }
}

fn log_err<T>(result: Result<T, impl Display>) {
    if let Err(e) = result {
        eprintln!("Error while logging: {}", e);
    }
}
