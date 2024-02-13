#![allow(unsafe_code)]

use crate::io::EnvironmentIo;
use crate::{io, Environment, HttpClient};
use std::fmt::Debug;
use std::sync::atomic::AtomicPtr;
use vrc_get_litedb::DatabaseConnection;

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

    fn connect(&self, io: &impl EnvironmentIo) -> io::Result<&DatabaseConnection> {
        if let Some(connection) = self.get() {
            return Ok(connection);
        }

        let db = io.connect_lite_db()?;

        let ptr = Box::into_raw(Box::new(db));

        match self.ptr.compare_exchange(
            std::ptr::null_mut(),
            ptr,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        ) {
            Ok(_) => {
                // success means the value is used so return the value
                Ok(unsafe { &*ptr })
            }
            Err(failure) => {
                // failure means the value is already set so drop the new value and return the old value
                // since it's not null
                let _ = unsafe { Box::from_raw(ptr) };
                Ok(unsafe { &*failure })
            }
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
    // TODO?: use inner mutability to get the database connection?
    pub(super) fn get_db(&self) -> io::Result<&DatabaseConnection> {
        self.litedb_connection.connect(&self.io)
    }
}
