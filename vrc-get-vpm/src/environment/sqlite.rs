use crate::io;
use crate::io::DefaultEnvironmentIo;
use parking_lot::Mutex;
use rusqlite::{Connection, ErrorCode, Transaction, TransactionBehavior};
use std::sync::Arc;
use tokio::task::spawn_blocking;

pub struct SQLiteConnection {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

static FILE_NAME: &str = "vrc-get/vrc-get.db";

impl SQLiteConnection {
    pub async fn connect(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let path = io.resolve(FILE_NAME.as_ref());

        spawn_blocking(|| {
            let mut connection = Connection::open(path).map_err(map_err)?;

            Self::initialize_database(&mut connection)?;

            Ok(Self {
                conn: Arc::new(Mutex::new(connection)),
            })
        })
        .await?
    }

    fn initialize_database(conn: &mut Connection) -> Result<(), io::Error> {
        loop {
            let mut tx = conn
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(map_err)?;
            match Self::initialize_database_inner(&mut tx) {
                Ok(()) => {
                    tx.commit().map_err(map_err)?;
                    return Ok(());
                }
                // SQLITE_BUSY likely means upgrading SHARED to RESERVED failed. retry from first.
                Err(err) if err.sqlite_error_code() == Some(ErrorCode::DatabaseBusy) => continue,
                Err(err) => return Err(map_err(err)),
            }
        }
    }

    fn initialize_database_inner(tx: &mut Transaction) -> Result<(), rusqlite::Error> {
        if !tx.table_exists(None, "migrations")? {
            tx.execute(
                "\
                CREATE TABLE IF NOT EXISTS migrations (\
                    id INTEGER PRIMARY KEY,\
                    name TEXT NOT NULL\
                )",
                (),
            )?;
        }

        let mut check_migration = tx.prepare("SELECT 1 from migrations WHERE id = ?")?;

        if check_migration.query([1])?.next()?.is_none() {
            // Note: unity_version_with_revision is in format of 2022.3.22f1 (012abcdef) or 2022.3.22f1
            tx.execute(
                "\
                CREATE TABLE projects (\
                    id INTEGER PRIMARY KEY,\
                    path TEXT NOT NULL UNIQUE,\
                    litedb_objectid TEXT UNIQUE,\
                    unity_version_with_revision TEXT NOT NULL,\
                    created_at UNIX SECONDS TIMESTAMP INTEGER NOT NULL,\
                    last_modified UNIX SECONDS TIMESTAMP INTEGER NOT NULL,\
                    type INTEGER NOT NULL,\
                    favorite BOOLEAN INTEGER NOT NULL,\
                    custom_unity_args JSON TEXT,\
                    unity_path TEXT,\
                    is_valid BOOLEAN INTEGER NOT NULL\
                )",
                (),
            )?;
            tx.execute(
                "INSERT INTO migrations (id, name) VALUES (?, ?)",
                (1, "initial"),
            )?;
        }

        Ok(())
    }
}

pub(crate) fn map_err(err: rusqlite::Error) -> io::Error {
    io::Error::other(err)
}
