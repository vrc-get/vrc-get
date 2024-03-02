use arc_swap::ArcSwapOption;
use chrono::format::StrftimeItems;
use log::{Log, Metadata, Record};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use serde::Serialize;
use std::fmt::{Display, Formatter};
use std::io::Write as _;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use vrc_get_vpm::io::{DefaultEnvironmentIo, EnvironmentIo};

static APP_HANDLE: ArcSwapOption<AppHandle> = ArcSwapOption::const_empty();

pub fn set_app_handle(handle: AppHandle) {
    APP_HANDLE.store(Some(Arc::new(handle)));
}

pub fn initialize_logger() {
    let env_io = DefaultEnvironmentIo::new_default();
    let log_folder = env_io.resolve("vrc-get-logs".as_ref());
    std::fs::create_dir_all(&log_folder).ok();
    let timestamp = chrono::Utc::now().format("%+");
    let log_file = log_folder.join(format!("vrc-get-{}.log", timestamp));
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(file) => {
            let logger = Logger {
                log_file: Some(Mutex::new(file)),
            };
            log::set_boxed_logger(Box::new(logger)).expect("error while setting logger");
            log::set_max_level(log::LevelFilter::Info);
            log::info!("logging to file {}", log_file.display());
        }
        Err(e) => {
            let logger = Logger { log_file: None };
            log::set_boxed_logger(Box::new(logger)).expect("error while setting logger");
            log::set_max_level(log::LevelFilter::Info);
            log::error!("error while opening log file: {}", e);
        }
    }
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
    time: chrono::DateTime<chrono::Utc>,
    level: LogLevel,
    target: String,
    message: String,
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
        const FORMAT: StrftimeItems = StrftimeItems::new("%+");

        write!(
            f,
            "{} [{: >5}] {}: {}",
            self.time.format_with_items(FORMAT),
            self.level,
            self.target,
            self.message
        )
    }
}

static LOG_BUFFER: Mutex<ConstGenericRingBuffer<LogEntry, 256>> =
    Mutex::new(ConstGenericRingBuffer::new());

struct Logger {
    log_file: Option<Mutex<std::fs::File>>,
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
        // log to console
        eprintln!("{}", entry);

        // log to file
        if let Some(log_file) = &self.log_file {
            let mut log_file = log_file.lock().unwrap();
            log_err(writeln!(log_file, "{}", entry));
        }

        // add to buffer
        {
            let mut buffer = LOG_BUFFER.lock().unwrap();
            buffer.push(entry.clone());
        }

        // log to tauri
        if let Some(app_handle) = APP_HANDLE.load().as_ref() {
            app_handle
                .emit_all("log", Some(entry))
                .expect("error while emitting log event");
        }
    }

    fn flush(&self) {
        if let Some(log_file) = &self.log_file {
            let mut log_file = log_file.lock().unwrap();
            log_err(log_file.flush())
        }
    }
}

fn log_err<T>(result: Result<T, impl Display>) {
    if let Err(e) = result {
        eprintln!("Error while logging: {}", e);
    }
}
