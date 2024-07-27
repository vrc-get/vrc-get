#![allow(unsafe_code)]

use crate::io::EnvironmentIo;
use crate::{io, Environment, HttpClient};
use std::fmt::Debug;
use std::sync::atomic::AtomicPtr;
use vrc_get_litedb::DatabaseConnection;

pub struct VccDatabaseConnection {
    pub(crate) connection: DatabaseConnection,
}

impl VccDatabaseConnection {
    pub fn connect(io: &impl EnvironmentIo) -> io::Result<Self> {
        Ok(Self {
            connection: io.connect_lite_db()?,
        })
    }

    pub async fn save(&self, _: &impl EnvironmentIo) -> io::Result<()> {
        // nop for now but might have to do something in the future
        Ok(())
    }

    pub(crate) fn db(&self) -> &DatabaseConnection {
        &self.connection
    }
}

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn load_from_db(&mut self, connection: &VccDatabaseConnection) -> io::Result<()> {
        self.settings.load_from_db(connection)
    }
}

pub(super) struct LiteDbConnectionHolder {
    ptr: AtomicPtr<DatabaseConnection>,
}

unsafe impl Send for LiteDbConnectionHolder {}

unsafe impl Sync for LiteDbConnectionHolder {}

impl Debug for LiteDbConnectionHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("LiteDbConnectionHolder")
            .field(&self.get())
            .finish()
    }
}

impl LiteDbConnectionHolder {
    pub(super) fn new() -> Self {
        Self {
            ptr: AtomicPtr::new(std::ptr::null_mut()),
        }
    }

    fn get(&self) -> Option<&DatabaseConnection> {
        let ptr = self.ptr.load(std::sync::atomic::Ordering::Acquire);
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }
}

impl Drop for LiteDbConnectionHolder {
    fn drop(&mut self) {
        let ptr = self.ptr.load(std::sync::atomic::Ordering::SeqCst);
        if !ptr.is_null() {
            let _ = unsafe { Box::from_raw(ptr) };
        }
    }
}

impl<T: HttpClient, IO: EnvironmentIo> Environment<T, IO> {
    pub fn disconnect_litedb(&mut self) {
        self.litedb_connection = LiteDbConnectionHolder::new();
    }
}
