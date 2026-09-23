use crate::environment::VccDatabaseConnection;
use crate::environment::settings::Settings;
use crate::environment::sqlite::{
    ArcMutexGuardConnectionExt, SQLiteConnection, datetime_from_unix,
};
use crate::io::{DefaultEnvironmentIo, DefaultProjectIo, IoTrait};
use crate::utils::{normalize_path, normalize_path_str};
use crate::version::UnityVersion;
use crate::{ProjectType, UnityProject, io};
use futures::future::join_all;
use hex::ToHex;
use log::{error, trace, warn};
use rusqlite::TransactionBehavior;
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::num::NonZeroI64;
use std::path::Path;
use tokio::task::spawn_blocking;
use vrc_get_litedb::bson::{DateTime, Document, ObjectId, Value};
use vrc_get_litedb::file_io::BsonAutoId;
use vrc_get_litedb::{bson, document};

#[derive(Debug)]
pub enum Error {
    LoadSettings(io::Error),
    LoadLitedb(io::Error),
    SQLite(rusqlite::Error),
    SaveSettings(io::Error),
    SaveLitedb(io::Error),
    LoadingProjectInfo(io::Error),
    BadPath(&'static str),
    AlreadyExists,
    ChangeProjectListNotSupported,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::LoadSettings(err) => write!(fmt, "Failed to load settings: {}", err),
            Error::LoadLitedb(err) => write!(fmt, "Failed to load litedb: {}", err),
            Error::SQLite(err) => write!(fmt, "SQLite error: {}", err),
            Error::SaveSettings(err) => write!(fmt, "Failed to save settings: {}", err),
            Error::SaveLitedb(err) => write!(fmt, "Failed to save litedb: {}", err),
            Error::LoadingProjectInfo(err) => write!(fmt, "Failed to load project info: {}", err),
            Error::BadPath(path) => write!(fmt, "Bad path: {}", path),
            Error::AlreadyExists => write!(fmt, "Specified project already exists"),
            Error::ChangeProjectListNotSupported => write!(
                fmt,
                "Adding / removing project is not supported for management session created with start_no_migration."
            ),
        }
    }
}

pub struct ProjectManagement<'io> {
    io: &'io DefaultEnvironmentIo,
    json: Option<Settings>,
    litedb: VccDatabaseConnection,
    sqlite: SQLiteConnection,
}

impl<'io> ProjectManagement<'io> {
    pub async fn start(io: &'io DefaultEnvironmentIo) -> Result<ProjectManagement<'io>, Error> {
        let mut json = Settings::load(io).await.map_err(Error::LoadSettings)?;
        let mut litedb = VccDatabaseConnection::connect(io)
            .await
            .map_err(Error::LoadLitedb)?;
        let mut sqlite = SQLiteConnection::connect(io).await.map_err(Error::SQLite)?;

        litedb.migrate(&json, io).await;
        litedb.dedup_projects();
        litedb.normalize_path();
        sqlite
            .sync_with_litedb(&mut litedb, io, SyncWithLitedbMode::TrustLitedb)
            .await?;

        json.load_from_db_inner(
            (litedb.db)
                .get_all(COLLECTION)
                .filter_map(|doc| doc[PATH].as_str())
                .collect::<HashSet<_>>(),
        );

        json.save(io).await.map_err(Error::SaveSettings)?;
        litedb.save(io).await.map_err(Error::SaveLitedb)?;

        Ok(ProjectManagement {
            io,
            json: Some(json),
            litedb,
            sqlite,
        })
    }

    /// Start project management without migrating from `settings.json`.
    ///
    /// Since this method does not touch `settings.json`, this session does not allow
    /// adding or removing projects, which requires updating `settings.json`.
    pub async fn start_no_migration(
        io: &'io DefaultEnvironmentIo,
    ) -> Result<ProjectManagement<'io>, Error> {
        let mut litedb = VccDatabaseConnection::connect(io)
            .await
            .map_err(Error::LoadLitedb)?;
        let mut sqlite = SQLiteConnection::connect(io).await.map_err(Error::SQLite)?;

        litedb.dedup_projects();
        litedb.normalize_path();
        sqlite
            .sync_with_litedb(&mut litedb, io, SyncWithLitedbMode::TrustLitedb)
            .await?;

        litedb.save(io).await.map_err(Error::SaveLitedb)?;

        Ok(ProjectManagement {
            io,
            json: None,
            litedb,
            sqlite,
        })
    }
}

macro_rules! check_absolute {
    ($project_path: ident, $function: ident, $result: expr) => {
        if !Path::new($project_path).is_absolute() {
            warn!(
                "{} is not an absolute path. {} does not support relative paths",
                $project_path,
                stringify!($function),
            );
            return $result;
        }
    };
}

struct GetProjectRunner;

impl GetProjectRunner {
    #[track_caller]
    fn prepare<'conn>(
        conn: &'conn rusqlite::Connection,
        // This query must starts with "WHERE" and not injection safe
        where_query: &str,
    ) -> rusqlite::Statement<'conn> {
        conn
            .prepare(&format!("SELECT id, path, litedb_objectid, unity_version_with_revision, created_at, last_modified, type, favorite, custom_unity_args, unity_path, is_valid FROM projects {where_query}"))
            .expect("Failed to prepare SQL statement")
    }

    fn read_row(row: &rusqlite::Row) -> UserProject {
        let id_col = 0;
        let path_col = 1;
        let litedb_objectid_col = 2;
        let unity_version_with_revision_col = 3;
        let created_at_col = 4;
        let last_modified_col = 5;
        let project_type_col = 6;
        let favorite_col = 7;
        let custom_unity_args_col = 8;
        let unity_path_col = 9;
        let is_valid_col = 10;

        let id = row.get(id_col).expect("id");
        let path = row.get(path_col).expect("path");
        let litedb_objectid = {
            row.get_ref(litedb_objectid_col)
                .and_then(|x| Ok(x.as_str_or_null()?))
                .expect("litedb_objectid")
                .and_then(|data| {
                    let mut parsed = [0u8; _];
                    hex::decode_to_slice(data, &mut parsed).ok()?;
                    Some(ObjectId::from_bytes(parsed))
                })
        };
        let (unity_version, unity_revision) = {
            let version = row
                .get_ref(unity_version_with_revision_col)
                .and_then(|x| Ok(x.as_str()?))
                .expect("unity_version");
            let (unity_version, revision) = version
                .split_once('(')
                .map(|(unity_version, revision)| {
                    (
                        unity_version.trim_end(),
                        Some(revision.trim_end_matches(')')),
                    )
                })
                .unwrap_or((version, None));

            (
                (!unity_version.is_empty())
                    .then(|| UnityVersion::parse(unity_version).expect("unparsable unity version")),
                revision.map(|x| x.to_string()),
            )
        };
        let created_at = row.get(created_at_col).expect("created_at");
        let last_modified = row.get(last_modified_col).expect("last_modified");
        let project_type = {
            ProjectType::from_i32(
                row.get_ref(project_type_col)
                    .and_then(|x| Ok(x.as_i64()?))
                    .expect("project_type") as i32,
            )
            .unwrap_or(ProjectType::Unknown)
        };
        let favorite = row.get(favorite_col).expect("favorite");
        let custom_unity_args = {
            row.get_ref(custom_unity_args_col)
                .and_then(|x| Ok(x.as_str_or_null()?))
                .expect("custom_unity_args")
                .map(|json| serde_json::from_str(json).expect("custom_unity_args json parse"))
        };
        let unity_path = row.get(unity_path_col).expect("unity_path");
        let is_valid = row.get(is_valid_col).expect("is_valid");

        UserProject {
            id,
            path,
            litedb_objectid,
            unity_version,
            unity_revision,
            created_at,
            last_modified,
            project_type,
            favorite,
            custom_unity_args,
            unity_path,
            is_valid,
        }
    }
}

impl<'io> ProjectManagement<'io> {
    pub fn get_projects(&self) -> Vec<UserProject> {
        let conn = (self.sqlite.conn).lock();
        let mut get_projects = GetProjectRunner::prepare(&conn, "");

        get_projects
            .query(())
            .expect("querying database")
            .map(|row| Ok(GetProjectRunner::read_row(row)))
            .collect::<Vec<_>>()
            .expect("failed to collect user projects")
    }

    /// It might be better to call `RealProjectInformation::load_from_fs` and
    /// then `sync_with_real_projects_information` since `load_from_fs` may take some time.
    pub async fn sync_with_real_projects(
        &mut self,
        skip_not_found: bool,
        io: &DefaultEnvironmentIo,
    ) -> Result<(), Error> {
        let projects = self.get_projects();

        let projects = join_all(projects.into_iter().map(|project| async {
            let path = project.path;
            match ValidRealProjectInformation::load_from_fs(io, path.clone()).await {
                Ok(Some(project)) => Some(RealProjectInformation::Valid(project)),
                Ok(None) => {
                    if !skip_not_found {
                        error!("Project {path} not found");
                    }
                    Some(RealProjectInformation::Invalid(
                        InvalidRealProjectInformation::new(path),
                    ))
                }
                Err(err) => {
                    error!("Error updating project information: {err}");
                    Some(RealProjectInformation::Invalid(
                        InvalidRealProjectInformation::new(path),
                    ))
                }
            }
        }))
        .await;

        self.sync_with_real_projects_informations(projects.into_iter().flatten().collect())
            .await
    }

    pub async fn sync_with_real_projects_information(
        &mut self,
        info: RealProjectInformation,
    ) -> Result<(), Error> {
        let conn = self.sqlite.conn.clone();
        spawn_blocking(move || {
            let mut conn = conn.lock_arc();
            let tx = conn.transaction().map_err(Error::SQLite)?;

            Self::sync_with_real_projects_information_impl(&tx, info)?;

            tx.commit().map_err(Error::SQLite)?;
            Ok(())
        })
        .await
        .unwrap()?;

        Ok(())
    }

    pub async fn sync_with_real_projects_informations(
        &mut self,
        information: Vec<RealProjectInformation>,
    ) -> Result<(), Error> {
        let conn = self.sqlite.conn.clone();
        spawn_blocking(move || {
            let mut conn = conn.lock_arc();
            let tx = conn.transaction().map_err(Error::SQLite)?;

            for info in information {
                Self::sync_with_real_projects_information_impl(&tx, info)?;
            }

            tx.commit().map_err(Error::SQLite)?;
            Ok(())
        })
        .await
        .unwrap()?;

        Ok(())
    }

    fn sync_with_real_projects_information_impl(
        tx: &rusqlite::Transaction,
        info: RealProjectInformation,
    ) -> Result<(), Error> {
        match info {
            RealProjectInformation::Valid(valid) => {
                let unity_with_revision = unity_version_with_revision(
                    valid.unity_version,
                    valid.unity_revision.as_deref(),
                );
                tx.prepare_cached("UPDATE projects SET is_valid = 1, unity_version_with_revision = ?, type = ? WHERE path = ?")
                    .expect("prepare")
                    .execute((unity_with_revision, valid.project_type as i32, &valid.path))
                    .map_err(Error::SQLite)?;
            }
            RealProjectInformation::Invalid(info) => {
                tx.prepare_cached("UPDATE projects SET is_valid = 0 WHERE path = ?")
                    .expect("prepare")
                    .execute([&info.path])
                    .map_err(Error::SQLite)?;
            }
        }
        Ok(())
    }

    fn find_project_bson(&self, project_path: &str) -> Option<Document> {
        (self.litedb.db)
            .get_by_index(COLLECTION, "Path", &project_path.into())
            .next()
            .cloned()
    }

    pub fn find_project(&self, project_path: &str) -> Result<Option<UserProject>, Error> {
        check_absolute!(project_path, find_project, Ok(None));
        let project_path = normalize_path_str(project_path);

        let conn = self.sqlite.conn.lock_arc();
        let mut stmt = GetProjectRunner::prepare(&conn, "WHERE path = ?");
        let mut rows = stmt.query((project_path,)).expect("query"); // the only source of error of query is ToSql Errors
        let Some(row) = rows.next().map_err(Error::SQLite)? else {
            return Ok(None);
        };

        Ok(Some(GetProjectRunner::read_row(row)))
    }

    pub async fn add_projects(&mut self, projects: &[UnityProject]) -> Result<(), Error> {
        let json = self
            .json
            .as_mut()
            .ok_or(Error::ChangeProjectListNotSupported)?;

        if projects
            .iter()
            .any(|proj| !proj.project_dir().is_absolute())
        {
            return Err(Error::BadPath("project path is not absolute"));
        }

        let project_types = join_all(projects.iter().map(|proj| proj.detect_project_type())).await;

        let mut new_projects = vec![];
        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;

        {
            let mut stmt = tx.prepare("INSERT INTO projects \
            (path, litedb_objectid, unity_version_with_revision, created_at, last_modified, type, favorite, is_valid) \
            VALUES (?, ?, ?, unixepoch('now'), unixepoch('now'), ?, 0, 1) \
            RETURNING (created_at)").unwrap();

            for (project, project_type) in projects.iter().zip(project_types) {
                let path = normalize_path(project.project_dir());
                let Some(path) = path.to_str() else {
                    return Err(Error::BadPath("project path is not utf8"));
                };
                let unity_version = project.unity_version();
                let unity_revision = project.unity_revision();

                let mut new_project =
                    UserProject::new(path.into(), Some(unity_version), project_type);
                let object_id = ObjectId::new();
                new_project.litedb_objectid = Some(object_id);

                if (self.litedb.db)
                    .get_by_index(COLLECTION, "Path", &Value::String(path.into()))
                    .next()
                    .is_some()
                {
                    return Err(Error::AlreadyExists);
                }

                let created_at = match stmt
                    .query((
                        path,
                        object_id.as_bytes().encode_hex::<String>(),
                        unity_version_with_revision(unity_version, unity_revision),
                        project_type as i32,
                    ))
                    .expect("query") // the only source of error of query is ToSql Errors
                    .next()
                    .map(|x| x.unwrap())
                {
                    Ok(row) => row.get::<_, i64>(0).unwrap(),
                    Err(e) if e.sqlite_extended_error_code() == Some(SQLITE_CONSTRAINT_UNIQUE) => {
                        return Err(Error::AlreadyExists);
                    }
                    Err(e) => return Err(Error::SQLite(e)),
                };
                new_projects.push(document! {
                    ID => object_id,
                    PATH => path,
                    UNITY_VERSION => unity_version.to_string(),
                    CREATED_AT => datetime_from_unix(created_at),
                    LAST_MODIFIED => datetime_from_unix(created_at),
                    TYPE => project_type as i32,
                    FAVORITE => false,
                });
                json.add_user_project(path);
            }
            stmt.finalize().unwrap();
        }

        (self.litedb.db)
            .insert(COLLECTION, new_projects, BsonAutoId::ObjectId)
            .expect("insert should never fail");

        tx.commit().map_err(Error::SQLite)?;

        json.save(self.io).await.map_err(Error::SaveSettings)?;
        self.litedb.save(self.io).await.map_err(Error::SaveLitedb)?;

        Ok(())
    }

    pub async fn remove_projects(&mut self, projects: &[UserProject]) -> Result<(), Error> {
        let json = self
            .json
            .as_mut()
            .ok_or(Error::ChangeProjectListNotSupported)?;

        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;

        {
            let mut stmt = tx.prepare("DELETE from projects WHERE id = ?").unwrap();

            for project in projects {
                if let Some(object_id) = project.litedb_objectid {
                    (self.litedb.db).delete(COLLECTION, &[Value::ObjectId(object_id)]);
                }
                if let Some(id) = project.id {
                    stmt.execute([id]).map_err(Error::SQLite)?;
                }

                json.remove_user_project(project.path());
            }
            stmt.finalize().unwrap();
        }

        tx.commit().map_err(Error::SQLite)?;
        json.save(self.io).await.map_err(Error::SaveSettings)?;
        self.litedb.save(self.io).await.map_err(Error::SaveLitedb)?;
        Ok(())
    }

    pub async fn set_favorite(
        &mut self,
        project_path: &str,
        favorite: bool,
    ) -> Result<bool, Error> {
        check_absolute!(project_path, set_favorite, Ok(false));
        let project_path = normalize_path_str(project_path);
        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;
        let changed = tx
            .prepare("UPDATE projects SET favorite = ? WHERE path = ?")
            .map_err(Error::SQLite)?
            .execute((favorite, &project_path))
            .map_err(Error::SQLite)?;
        tx.commit().map_err(Error::SQLite)?;

        if changed != 0
            && let Some(mut project) = self.find_project_bson(&project_path)
        {
            project.insert(FAVORITE, favorite);

            (self.litedb.db)
                .update(COLLECTION, vec![project])
                .expect("update");

            self.litedb.save(self.io).await.map_err(Error::SaveLitedb)?;
        }
        Ok(changed != 0)
    }

    pub async fn set_custom_unity_args(
        &mut self,
        project_path: &str,
        args: Option<&[String]>,
    ) -> Result<bool, Error> {
        check_absolute!(project_path, set_custom_unity_args, Ok(false));
        let project_path = normalize_path_str(project_path);
        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;
        let changed = tx
            .prepare("UPDATE projects SET custom_unity_args = ? WHERE path = ?")
            .map_err(Error::SQLite)?
            .execute((
                args.map(|x| serde_json::to_string(x).unwrap()),
                &project_path,
            ))
            .map_err(Error::SQLite)?;
        tx.commit().map_err(Error::SQLite)?;
        // No need to update litedb since this is a vrc-get-only attribute
        Ok(changed != 0)
    }

    pub async fn set_unity_path(
        &mut self,
        project_path: &str,
        unity_path: Option<&str>,
    ) -> Result<bool, Error> {
        check_absolute!(project_path, set_custom_unity_args, Ok(false));
        let project_path = normalize_path_str(project_path);

        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;
        let changed = tx
            .prepare("UPDATE projects SET unity_path = ? WHERE path = ?")
            .map_err(Error::SQLite)?
            .execute((
                unity_path.map(|x| serde_json::to_string(x).unwrap()),
                project_path,
            ))
            .map_err(Error::SQLite)?;
        tx.commit().map_err(Error::SQLite)?;
        // No need to update litedb since this is a vrc-get-only attribute
        Ok(changed != 0)
    }

    pub async fn update_project_last_modified(
        &mut self,
        project_path: &str,
    ) -> Result<bool, Error> {
        check_absolute!(project_path, set_custom_unity_args, Ok(false));
        let project_path = normalize_path_str(project_path);

        let tx = (self.sqlite.conn.lock_arc())
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(Error::SQLite)?;
        let updated_date = tx
            .prepare("UPDATE projects SET last_modified = unixepoch('now') WHERE path = ? RETURNING last_modified")
            .map_err(Error::SQLite)?
            .query((&project_path,))
            .map_err(Error::SQLite)?
            .next()
            .map_err(Error::SQLite)?
            .map(|row| row.get::<_, i64>(0).unwrap());
        tx.commit().map_err(Error::SQLite)?;

        if let Some(updated_date) = updated_date
            && let Some(mut project) = self.find_project_bson(&project_path)
        {
            project.insert(LAST_MODIFIED, datetime_from_unix(updated_date));
            (self.litedb.db)
                .update(COLLECTION, vec![project])
                .expect("update");

            self.litedb.save(self.io).await.map_err(Error::SaveLitedb)?;
        }

        Ok(updated_date.is_some())
    }
}

pub(crate) static COLLECTION: &str = "projects";
pub(crate) static ID: &str = "_id";
pub(crate) static PATH: &str = "Path";
pub(crate) static UNITY_VERSION: &str = "UnityVersion";
pub(crate) static TYPE: &str = "Type";
pub(crate) static FAVORITE: &str = "Favorite";
pub(crate) static CREATED_AT: &str = "CreatedAt";
pub(crate) static LAST_MODIFIED: &str = "LastModified";

pub(crate) static VRC_GET: &str = "vrc-get";
pub(crate) static CACHED_UNITY_REVISION: &str = "cached_unity_version";
pub(crate) static UNITY_REVISION: &str = "unity_revision";
pub(crate) static CUSTOM_UNITY_ARGS: &str = "custom_unity_args";
pub(crate) static UNITY_PATH: &str = "unity_path";
pub(crate) static IS_VALID: &str = "is_valid";

impl VccDatabaseConnection {
    async fn migrate(&mut self, settings: &Settings, io: &DefaultEnvironmentIo) {
        let Some(setting_projects) = settings.user_projects() else {
            // The userProjects key is absent in settings.json.
            // The vcc.litedb is the single source of truth.
            return;
        };
        let projects = setting_projects
            .iter()
            .filter(|x| {
                if Path::new(x.as_ref()).is_absolute() {
                    true
                } else {
                    error!("Skipping relative path: {x}");
                    false
                }
            })
            .map(|x| x.as_ref())
            .collect::<HashSet<_>>();

        let db_projects = self.db.get_all(COLLECTION).cloned().collect::<Vec<_>>();

        let db_projects_by_path = db_projects
            .iter()
            .filter_map(|x| Some((x.get("Path").as_str()?, x)))
            .collect::<HashMap<_, _>>();

        let mut to_insert = vec![];

        // add new projects
        for project in &projects {
            if !db_projects_by_path.contains_key(*project) {
                async fn get_project_type(
                    io: &DefaultEnvironmentIo,
                    path: &Path,
                ) -> io::Result<(ProjectType, Option<UnityVersion>, Option<String>)>
                {
                    let project =
                        UnityProject::load(DefaultProjectIo::new(io.resolve(path).into())).await?;
                    let detected_type = project.detect_project_type().await;
                    Ok((
                        detected_type,
                        Some(project.unity_version()),
                        project.unity_revision().map(|x| x.to_owned()),
                    ))
                }
                let (project_type, unity_version, unity_revision) = get_project_type(
                    io,
                    project.as_ref(),
                )
                .await
                .unwrap_or((ProjectType::Unknown, None, None));
                let mut project = UserProject::new((*project).into(), unity_version, project_type);
                if unity_version.is_some() {
                    project.unity_revision = unity_revision;
                }
                to_insert.push(project);
            }
        }

        if !to_insert.is_empty() {
            self.db
                .insert(
                    COLLECTION,
                    to_insert.iter().map(|x| x.to_bson()).collect(),
                    BsonAutoId::ObjectId,
                )
                .expect("inserting document");
        }

        let mut ids_to_delete = vec![];

        // remove deleted projects
        for project in db_projects.iter() {
            if project[PATH]
                .as_str()
                .map(|path| !projects.contains(path))
                .unwrap_or(true)
            {
                ids_to_delete.push(project.get(ID).clone());
            }
        }

        self.db.delete(COLLECTION, &ids_to_delete);
    }
}

impl VccDatabaseConnection {
    fn normalize_path(&mut self) {
        let mut to_update = vec![];

        for project in self.db.get_all(COLLECTION) {
            let Some(path) = project[PATH].as_str() else {
                continue;
            };

            let normalized = normalize_path(path.as_ref());
            if normalized.to_str().unwrap() != path {
                let mut project = project.clone();
                project.insert(PATH, normalized.into_os_string().into_string().unwrap());

                to_update.push(project);
            }
        }

        if !to_update.is_empty() {
            self.db
                .update(COLLECTION, to_update)
                .expect("updating project");
        }
    }

    fn dedup_projects(&mut self) {
        let projects = self.db.get_all(COLLECTION).collect::<Vec<_>>();

        let mut projects_by_path = HashMap::<_, Vec<_>>::new();

        for project in projects {
            if let Some(path) = project[PATH].as_str() {
                projects_by_path
                    .entry(path.to_string())
                    .or_default()
                    .push(project);
            }
        }

        let mut updates = vec![];
        let mut deletes = vec![];

        for (_, values) in projects_by_path {
            if values.len() == 1 {
                continue;
            }

            // update favorite and last modified

            let favorite = values
                .iter()
                .any(|x| x[FAVORITE].as_bool().unwrap_or(false));
            let created_at = values
                .iter()
                .filter_map(|x| x[CREATED_AT].as_date_time())
                .min()
                .unwrap();
            let last_modified = values
                .iter()
                .filter_map(|x| x[LAST_MODIFIED].as_date_time())
                .max()
                .unwrap();

            let mut values_iter = values.into_iter();
            let mut project = Cow::Borrowed(values_iter.next().unwrap());
            if project[FAVORITE].as_bool() != Some(favorite) {
                project.to_mut().insert(FAVORITE, favorite);
            }
            if project[LAST_MODIFIED].as_date_time() != Some(last_modified) {
                project.to_mut().insert(LAST_MODIFIED, last_modified);
            }
            if project[CREATED_AT].as_date_time() != Some(created_at) {
                project.to_mut().insert(CREATED_AT, created_at);
            }

            if let Cow::Owned(project) = project {
                updates.push(project);
            }

            // remove rest
            for project in values_iter {
                deletes.push(project[ID].clone());
            }
        }

        self.db.update(COLLECTION, updates).expect("update");
        self.db.delete(COLLECTION, &deletes);
    }
}

/// This enum specifies the mode for synchronizing projects between SQLite and LiteDB.
///
/// For historical reasons, vrc-get has two sources of the project list: litedb, which is list
/// of VCC, and SQLite, which is the vrc-get specific database.
///
/// Therefore, we need to synchronize the project list between SQLite and LiteDB as necessary.
/// This enum specifies the mode for synchronizing projects between SQLite and LiteDB.
///
/// In both modes, projects that do not have `litedb_objectid` column will not be removed.
/// `litedb_objectid` will be set when projects are live in LiteDB.
#[derive(Default, Debug, Copy, Clone)]
pub enum SyncWithLitedbMode {
    /// Add projects from litedb to SQLite, and then add projects from SQLite to LiteDB.
    /// In this mode, projects from either database will be live.
    ///
    /// The benefit of this mode is that we can work around the VCC's behavior that removes
    /// projects that are not accessible to VCC, which can happen unintentionally with network
    /// drives or removable drives, or permission errors.
    ///
    /// The downsides of this mode are that removing projects from VCC would not be reflected in SQLite
    /// and rolled back to the previous state.
    ///
    /// This is the default mode since we think removing projects unintentionally is problematic
    /// than restoring projects unintentionally.
    #[default]
    ProjectsUnion,
    /// Trust LiteDB for the project list and always reflect which are exists in LiteDB.
    /// This mode also overwrites most metadata of projects in SQLite with LiteDB.
    ///
    /// The benefit of this mode is that removing projects from VCC would be preserved.
    ///
    /// The downsides of this mode are that VCC's unintentional removal of projects.
    ///
    /// Users may prefer this mode when VCC is the primary tool for project management.
    TrustLitedb,
}

impl SQLiteConnection {
    async fn sync_with_litedb(
        &mut self,
        litedb: &mut VccDatabaseConnection,
        io: &DefaultEnvironmentIo,
        mode: SyncWithLitedbMode,
    ) -> Result<(), Error> {
        let litedb_projects = (litedb.db).get_all(COLLECTION).cloned().collect::<Vec<_>>();
        let conn = self.conn.clone();
        let projects_to_insert = spawn_blocking(move || {
            let tx = conn
                .lock_arc()
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(Error::SQLite)?;

            let mut inserted_projects = BTreeSet::new();

            // Copy projects from LiteDB to SQLite with upsert
            {
                let mut upsert;
                match mode {
                    SyncWithLitedbMode::ProjectsUnion => {
                        upsert = tx.prepare("INSERT INTO projects \
                            (path, litedb_objectid, unity_version_with_revision, created_at, last_modified, type, favorite, custom_unity_args, unity_path, is_valid)\
                            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)\
                        ON CONFLICT (path) DO UPDATE SET litedb_objectid = excluded.litedb_objectid")
                            .unwrap();
                    }
                    SyncWithLitedbMode::TrustLitedb => {
                        upsert = tx.prepare("INSERT INTO projects \
                            (path, litedb_objectid, unity_version_with_revision, created_at, last_modified, type, favorite, custom_unity_args, unity_path, is_valid)\
                            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)\
                        ON CONFLICT (path) DO UPDATE SET \
                            litedb_objectid = excluded.litedb_objectid, \
                            unity_version_with_revision = excluded.unity_version_with_revision, \
                            created_at = excluded.created_at, \
                            last_modified = excluded.last_modified, \
                            type = excluded.type, \
                            favorite = excluded.favorite\
                            ")
                            .unwrap();
                    }
                }

                for l_proj in &litedb_projects {
                    if let Some(object_id) = l_proj.get(ID).as_object_id()
                        && let Some(unity_version) = UnityVersion::parse(l_proj[UNITY_VERSION].as_str().unwrap())
                        && let Some(path) = l_proj[PATH].as_str()
                        && let Some(created_at) = l_proj[CREATED_AT].as_date_time()
                        && let Some(last_modified) = l_proj[LAST_MODIFIED].as_date_time()
                        && let Some(project_type) = l_proj[TYPE].as_i32()
                    {
                        let favorite = l_proj[FAVORITE].as_bool().unwrap_or(false);
                        let unity_revision = {
                            if let Some(unity_version) = l_proj[UNITY_VERSION].as_str() {
                                l_proj
                                    .get(VRC_GET)
                                    .as_document()
                                    .filter(|x| x[CACHED_UNITY_REVISION].as_str() == Some(unity_version))
                                    .and_then(|x| x[UNITY_REVISION].as_str())
                            } else {
                                None
                            }
                        };
                        let unity_version_with_revision = if let Some(revision) = unity_revision {
                            format!("{unity_version}({revision})")
                        } else {
                            unity_version.to_string()
                        };
                        let custom_unity_args = l_proj
                            .get(VRC_GET)
                            .as_document()
                            .and_then(|x| x[CUSTOM_UNITY_ARGS].as_array())
                            .and_then(|x| {
                                x.as_slice()
                                    .iter()
                                    .map(|x| x.as_str().map(|x| x.to_owned()))
                                    .collect::<Option<Vec<_>>>()
                            });
                        let unity_path = l_proj[VRC_GET]
                            .as_document()
                            .and_then(|x| x[UNITY_PATH].as_str());
                        let is_valid_project = l_proj
                            .get(VRC_GET)
                            .as_document()
                            .and_then(|x| x[IS_VALID].as_bool());
                        // TODO: tx.set_last_insert_rowid(0); instead
                        let pre_last_insert_rowid = tx.last_insert_rowid();
                        upsert.execute((
                            path,
                            object_id.as_bytes().encode_hex::<String>(),
                            unity_version_with_revision,
                            created_at.as_unix_milliseconds() / 1000,
                            last_modified.as_unix_milliseconds() / 1000,
                            ProjectType::from_i32(project_type).unwrap_or(ProjectType::Unknown) as i32,
                            favorite,
                            custom_unity_args.map(|x| serde_json::to_string(&x).unwrap()),
                            unity_path,
                            is_valid_project.unwrap_or(true),
                        )).map_err(Error::SQLite)?;
                        let last_insert_rowid = tx.last_insert_rowid();
                        if last_insert_rowid != pre_last_insert_rowid {
                            trace!("Project inserted into SQLite: ({path}, {object_id:?})");
                            inserted_projects.insert(last_insert_rowid);
                        }
                    }
                }
            }

            let mut projects_to_insert: Vec<Document> = vec![];
            match mode {
                SyncWithLitedbMode::ProjectsUnion => {
                    // Copy projects from SQLite to LiteDB
                    let mut l_proj_by_id = litedb_projects.into_iter().flat_map(|x| Some((x.get(ID).as_object_id()?, x))).collect::<HashMap<_, _>>();
                    let id_by_path = l_proj_by_id.iter().flat_map(|(&id, proj)| Some((proj[PATH].as_str()?.to_string(), id))).collect::<BTreeMap<_, _>>();

                    let mut get_projects = tx.prepare("SELECT \
                            id, path, litedb_objectid, unity_version_with_revision, created_at, last_modified, type, favorite, custom_unity_args, unity_path, is_valid \
                            FROM projects")
                        .unwrap();
                    let mut update_object_id = tx.prepare("UPDATE projects SET litedb_objectid = ? WHERE id = ?").unwrap();
                    let id_col = get_projects.column_index("id").unwrap();
                    let path_col = get_projects.column_index("path").unwrap();
                    let litedb_objectid_col = get_projects.column_index("litedb_objectid").unwrap();
                    let unity_version_with_revision_col = get_projects.column_index("unity_version_with_revision").unwrap();
                    let created_at_col = get_projects.column_index("created_at").unwrap();
                    let last_modified_col = get_projects.column_index("last_modified").unwrap();
                    let project_type_col = get_projects.column_index("type").unwrap();
                    let favorite_col = get_projects.column_index("favorite").unwrap();

                    let mut query = get_projects.query(()).unwrap();
                    while let Some(row) = query.next().map_err(Error::SQLite)? {
                        let id = row.get::<_, i64>(id_col).unwrap();
                        if inserted_projects.contains(&id) {
                            continue;
                        }
                        let object_id = row.get_ref(litedb_objectid_col).unwrap().as_bytes_or_null().expect("litedb_objectid as bytes");
                        let object_id = if let Some(object_id_hex) = object_id {
                            let mut object_id_bytes = [0u8; _];
                            if hex::decode_to_slice(object_id_hex, &mut object_id_bytes).is_ok()
                            {
                                Some(bson::ObjectId::from_bytes(object_id_bytes))
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        let path = row.get_ref(path_col).unwrap().as_str().expect("path as str");

                        if let Some(object_id) = object_id
                            && let Some(existing) = l_proj_by_id.get_mut(&object_id)
                            && let Some(existing_path) = existing[PATH].as_str()
                            && existing_path == path {
                            // Good News! the project already exists in the database! with the correct path
                            trace!("Project already exists in the database with the correct path: ({path}, {object_id:?}): {:?}", existing);
                        } else if let Some(object_id) = id_by_path.get(path) {
                            // Not bad, the project was not found with the id, but found by path
                            trace!("Project already exists in the database by the path: ({path}, {object_id:?}): {:?}", l_proj_by_id[object_id]);
                            update_object_id.execute((object_id.as_bytes().encode_hex::<String>(), id)).map_err(Error::SQLite)?;
                        } else {
                            // There is no project in the database with the given id or path
                            // We need to insert this project to litedb
                            trace!("Project does not exist in the database with the given id or path");
                            let unity_version = row.get_ref(unity_version_with_revision_col).unwrap().as_str().unwrap();
                            let unity_version = unity_version.split_once('(').map(|(v, _)| v.trim_end()).unwrap_or(unity_version);
                            let created_at = row.get_ref(created_at_col).unwrap().as_i64().unwrap();
                            let last_modified = row.get_ref(last_modified_col).unwrap().as_i64().unwrap();
                            let project_type = row.get_ref(project_type_col).unwrap().as_i64().unwrap();
                            let favorite = row.get::<_, bool>(favorite_col).unwrap();

                            let litedb_id = bson::ObjectId::new();

                            projects_to_insert.push(document! {
                                ID => litedb_id,
                                PATH => path,
                                UNITY_VERSION => unity_version,
                                CREATED_AT => datetime_from_unix(created_at),
                                LAST_MODIFIED => datetime_from_unix(last_modified),
                                TYPE => project_type as i32,
                                FAVORITE => favorite,
                            });

                            update_object_id.execute((litedb_id.as_bytes().encode_hex::<String>(), id)).map_err(Error::SQLite)?;
                        }
                    }
                }
                SyncWithLitedbMode::TrustLitedb => {
                    // remove projects does not exist in LiteDB
                    let mut stmt = tx.prepare("DELETE FROM projects WHERE litedb_objectid IS NOT NULL AND path NOT IN (SELECT value from json_each(?)) RETURNING projects.path").unwrap();
                    let mut rows = stmt.query((serde_json::to_string(&litedb_projects.iter().flat_map(|x| x[PATH].as_str()).collect::<Vec<&str>>()).unwrap(),)).unwrap();
                    while let Some(rows) = rows.next().map_err(Error::SQLite)? {
                        trace!("removed {}", rows.get::<_, String>(0).unwrap())
                    }
                }
            }

            tx.commit().unwrap();

            Ok(projects_to_insert)
        })
            .await
            .unwrap()?;

        litedb
            .db
            .insert(COLLECTION, projects_to_insert, BsonAutoId::ObjectId)
            .expect("insert");

        litedb.save(io).await.map_err(Error::SaveLitedb)?;

        Ok(())
    }
}

/// The Data Structure to store information required for updating information in the Database
pub enum RealProjectInformation {
    Valid(ValidRealProjectInformation),
    Invalid(InvalidRealProjectInformation),
}

pub struct InvalidRealProjectInformation {
    path: String,
}

pub struct ValidRealProjectInformation {
    path: String,
    unity_version: UnityVersion,
    unity_revision: Option<String>,
    project_type: ProjectType,
}

impl RealProjectInformation {
    pub fn path(&self) -> &str {
        match self {
            RealProjectInformation::Valid(i) => i.path(),
            RealProjectInformation::Invalid(i) => i.path(),
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            RealProjectInformation::Valid(_) => true,
            RealProjectInformation::Invalid(_) => false,
        }
    }
}

impl InvalidRealProjectInformation {
    pub fn new(path: String) -> InvalidRealProjectInformation {
        InvalidRealProjectInformation { path }
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl ValidRealProjectInformation {
    pub async fn load_from_fs(io: &DefaultEnvironmentIo, path: String) -> io::Result<Option<Self>> {
        if !io.is_dir(path.as_ref()).await {
            return Ok(None);
        }

        let loaded_project = UnityProject::load(io.new_project_io(path.as_ref())).await?;
        let unity_version = loaded_project.unity_version();
        let unity_revision = loaded_project.unity_revision().map(ToOwned::to_owned);
        let project_type = loaded_project.detect_project_type().await;

        Ok(Some(ValidRealProjectInformation {
            path,
            unity_version,
            unity_revision,
            project_type,
        }))
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn unity_version(&self) -> UnityVersion {
        self.unity_version
    }

    pub fn unity_revision(&self) -> Option<&str> {
        self.unity_revision.as_deref()
    }

    pub fn project_type(&self) -> ProjectType {
        self.project_type
    }
}

pub struct UserProject {
    id: Option<NonZeroI64>, // 0 means unsigned
    path: String,
    litedb_objectid: Option<ObjectId>,
    unity_version: Option<UnityVersion>,
    unity_revision: Option<String>,
    created_at: i64,    // unix timestamp in seconds
    last_modified: i64, // unix timestamp in seconds
    project_type: ProjectType,
    favorite: bool,
    custom_unity_args: Option<Vec<String>>,
    unity_path: Option<String>,
    is_valid: bool,
}

impl UserProject {
    pub(crate) fn to_bson(&self) -> Document {
        let mut doc = document! {
            PATH => &self.path,
            UNITY_VERSION => self.unity_version.map(|x| x.to_string()),
            CREATED_AT => datetime_from_unix(self.created_at),
            LAST_MODIFIED => datetime_from_unix(self.last_modified),
            TYPE => self.project_type as i32,
            FAVORITE => self.favorite,
        };
        if let Some(object_id) = self.litedb_objectid {
            doc.insert(ID, object_id);
        }
        doc
    }
}

fn unity_version_with_revision(
    unity_version: UnityVersion,
    unity_revision: Option<&str>,
) -> String {
    if let Some(revision) = unity_revision {
        format!("{}({})", unity_version, revision)
    } else {
        unity_version.to_string()
    }
}

impl UserProject {
    fn new(path: Box<str>, unity_version: Option<UnityVersion>, project_type: ProjectType) -> Self {
        let now = DateTime::now().as_unix_milliseconds() / 1000;
        Self {
            id: None,
            path: path.into_string(),
            litedb_objectid: None,
            unity_version,
            unity_revision: None,
            created_at: now,
            last_modified: now,
            project_type,
            favorite: false,
            custom_unity_args: None,
            unity_path: None,
            is_valid: true,
        }
    }

    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub fn name(&self) -> &str {
        self.path()
            .rsplit_once(cfg_select! {
                windows => ['/', '\\'],
                _ => ['/'],
            })
            .map(|(_, name)| name)
            .unwrap_or(self.path())
    }

    pub fn crated_at(&self) -> DateTime {
        datetime_from_unix(self.created_at)
    }

    pub fn last_modified(&self) -> DateTime {
        datetime_from_unix(self.last_modified)
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.unity_version
    }

    pub fn project_type(&self) -> ProjectType {
        self.project_type
    }

    pub fn favorite(&self) -> bool {
        self.favorite
    }

    pub fn unity_revision(&self) -> Option<&str> {
        self.unity_revision.as_deref()
    }

    pub fn custom_unity_args(&self) -> Option<&[String]> {
        self.custom_unity_args.as_deref()
    }

    pub fn unity_path(&self) -> Option<&str> {
        self.unity_path.as_deref()
    }

    pub fn is_valid_project(&self) -> Option<bool> {
        Some(self.is_valid)
    }
}
