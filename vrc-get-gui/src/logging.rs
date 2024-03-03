use arc_swap::ArcSwapOption;
use log::{Log, Metadata, Record};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::io::Write as _;
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Manager};
use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo};

static APP_HANDLE: ArcSwapOption<AppHandle> = ArcSwapOption::const_empty();

pub fn set_app_handle(handle: AppHandle) {
    APP_HANDLE.store(Some(Arc::new(handle)));
}

pub fn initialize_logger() {
    let (sender, receiver) = mpsc::channel::<LogChannelMessage>();
    let logger = Logger { sender };

    log::set_max_level(log::LevelFilter::Info);
    log::set_boxed_logger(Box::new(logger)).expect("error while setting logger");

    start_logging_thread(receiver);
}

fn start_logging_thread(receiver: mpsc::Receiver<LogChannelMessage>) {
    let env_io = DefaultEnvironmentIo::new_default();
    let log_folder = env_io.resolve("vrc-get-logs".as_ref());
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
    time: chrono::DateTime<chrono::Utc>,
    level: LogLevel,
    target: String,
    message: String,
}

fn to_rfc3339_micros<S>(
    time: &chrono::DateTime<chrono::Utc>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    time.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
        .serialize(serializer)
}

impl LogEntry {
    pub fn new(record: &Record) -> Self {
        LogEntry {
            time: chrono::Utc::now(),
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
                .to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
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
