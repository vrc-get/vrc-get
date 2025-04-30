use crate::environment::VccDatabaseConnection;
use crate::environment::settings::Settings;
use crate::io::{DefaultEnvironmentIo, DefaultProjectIo, IoTrait};
use crate::utils::{check_absolute_path, normalize_path};
use crate::version::UnityVersion;
use crate::{ProjectType, UnityProject, io};
use futures::future::join_all;
use log::error;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::Path;
use vrc_get_litedb::bson::{Array, DateTime, Document, Value};
use vrc_get_litedb::document;
use vrc_get_litedb::file_io::BsonAutoId;

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

impl VccDatabaseConnection {
    pub async fn migrate(
        &mut self,
        settings: &Settings,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<()> {
        let projects = settings
            .user_projects()
            .iter()
            .filter(|x| {
                if Path::new(x.as_ref()).is_absolute() {
                    true
                } else {
                    error!("Skipping relative path: {}", x);
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
                    let detected_type = project.detect_project_type().await?;
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
                if let Some(unity) = unity_version {
                    project.set_unity_revision(unity, unity_revision);
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

        Ok(())
    }
}

impl VccDatabaseConnection {
    pub async fn sync_with_real_projects(
        &mut self,
        skip_not_found: bool,
        io: &DefaultEnvironmentIo,
    ) -> io::Result<()> {
        let projects = self.db.get_all(COLLECTION).collect::<Vec<_>>();

        let changed_projects = join_all(
            projects
                .into_iter()
                .map(|x| update_project_with_actual_data(io, x, skip_not_found)),
        )
        .await;

        self.db
            .update(COLLECTION, changed_projects.into_iter().flatten().collect())
            .expect("updating project");

        async fn update_project_with_actual_data(
            io: &DefaultEnvironmentIo,
            project: &Document,
            skip_not_found: bool,
        ) -> Option<Document> {
            match update_project_with_actual_data_inner(io, project, skip_not_found).await {
                Ok(Some(project)) => Some(project),
                Ok(None) => None,
                Err(err) => {
                    error!("Error updating project information: {}", err);
                    None
                }
            }
        }

        async fn update_project_with_actual_data_inner(
            io: &DefaultEnvironmentIo,
            project: &Document,
            skip_not_found: bool,
        ) -> io::Result<Option<Document>> {
            let mut project = Cow::Borrowed(project);

            let Some(path) = project[PATH].as_str() else {
                return Ok(None);
            };
            let path = Path::new(path);

            if !io.is_dir(path).await {
                if !skip_not_found {
                    error!("Project {} not found", path.display());
                }
                return Ok(None);
            }

            let normalized = normalize_path(path);
            let normalized = if normalized != path {
                Some(normalized)
            } else {
                None
            };

            let loaded_project = UnityProject::load(io.new_project_io(path)).await?;
            {
                let unity_version = loaded_project.unity_version();
                let unity_version = unity_version.to_string();
                if let Some(revision) = loaded_project.unity_revision() {
                    if Some(unity_version.as_str()) != project[UNITY_VERSION].as_str()
                        || Some(revision)
                            != project[VRC_GET]
                                .as_document()
                                .filter(|x| {
                                    x.get(CACHED_UNITY_REVISION).as_str()
                                        == Some(unity_version.as_str())
                                })
                                .and_then(|x| x.get(UNITY_REVISION).as_str())
                    {
                        project
                            .to_mut()
                            .insert(UNITY_VERSION, unity_version.clone());
                        let vrc_get = project.to_mut().entry(VRC_GET).document_or_replace();
                        vrc_get.insert(UNITY_REVISION, revision);
                        vrc_get.insert(CACHED_UNITY_REVISION, unity_version);
                    }
                } else {
                    #[allow(clippy::collapsible_else_if)]
                    if Some(unity_version.as_str()) != project[UNITY_VERSION].as_str() {
                        project.to_mut().insert(UNITY_VERSION, unity_version);
                    }
                }
            }

            let project_type = loaded_project.detect_project_type().await?;
            if project[TYPE].as_i32() != Some(project_type as i32) {
                project.to_mut().insert(TYPE, project_type as i32);
            }

            if let Some(normalized) = normalized {
                project.to_mut().insert(PATH, normalized.to_str().unwrap());
            }

            Ok(match project {
                Cow::Owned(o) => Some(o),
                Cow::Borrowed(_) => None,
            })
        }

        Ok(())
    }

    pub fn dedup_projects(&mut self) {
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

    pub fn get_projects(&self) -> Vec<UserProject> {
        self.db
            .get_all(COLLECTION)
            .cloned()
            .map(UserProject::from_document)
            .collect::<Vec<_>>()
    }

    pub fn find_project_bson(&self, project_path: &str) -> io::Result<Option<&Document>> {
        check_absolute_path(project_path)?;
        let project_path = normalize_path(project_path.as_ref());

        Ok(self
            .db
            .get_by_index(COLLECTION, "Path", &project_path.to_str().unwrap().into())
            .next())
    }

    pub fn find_project(&self, project_path: &str) -> io::Result<Option<UserProject>> {
        Ok(self
            .find_project_bson(project_path)?
            .cloned()
            .map(UserProject::from_document))
    }

    pub fn update_project_last_modified(&mut self, project_path: &str) -> io::Result<()> {
        check_absolute_path(project_path)?;
        let Some(mut project) = self.find_project_bson(project_path)?.cloned() else {
            return Ok(());
        };

        project.insert(LAST_MODIFIED, DateTime::now());
        self.db.update(COLLECTION, vec![project]).expect("update");
        Ok(())
    }

    pub fn update_project(&mut self, project: &UserProject) {
        self.db
            .update(COLLECTION, vec![project.to_bson()])
            .expect("update");
    }

    pub fn remove_project(&mut self, project: &UserProject) {
        self.db.delete(COLLECTION, &[project.bson[ID].clone()]);
    }

    pub async fn add_project(&mut self, project: &UnityProject) -> io::Result<()> {
        check_absolute_path(project.project_dir())?;
        let path = normalize_path(project.project_dir());
        let path = path.to_str().ok_or(io::Error::new(
            io::ErrorKind::InvalidData,
            "project path is not utf8",
        ))?;
        let unity_version = project.unity_version();
        let unity_revision = project.unity_revision();

        let project_type = project.detect_project_type().await?;

        let mut new_project = UserProject::new(path.into(), Some(unity_version), project_type);
        new_project.set_unity_revision(unity_version, unity_revision.map(ToOwned::to_owned));

        self.db
            .insert(
                COLLECTION,
                vec![new_project.to_bson()],
                BsonAutoId::ObjectId,
            )
            .expect("insert");

        Ok(())
    }
}

pub struct UserProject {
    bson: Document,
}

impl UserProject {
    pub(crate) fn to_bson(&self) -> Document {
        self.bson.clone()
    }
}

impl UserProject {
    fn new(path: Box<str>, unity_version: Option<UnityVersion>, project_type: ProjectType) -> Self {
        let now = DateTime::now();
        Self {
            bson: document! {
                PATH => path.as_ref(),
                UNITY_VERSION => unity_version.as_ref().map(ToString::to_string),
                CREATED_AT => now,
                LAST_MODIFIED => now,
                TYPE => project_type as i32,
                FAVORITE => false,
            },
        }
    }

    fn from_document(document: Document) -> Self {
        Self { bson: document }
    }

    pub fn path(&self) -> Option<&str> {
        self.bson[PATH].as_str()
    }

    pub fn name(&self) -> Option<&str> {
        self.path()
            .map(Path::new)
            .and_then(Path::file_name)
            .and_then(OsStr::to_str)
    }

    pub fn crated_at(&self) -> Option<DateTime> {
        self.bson[CREATED_AT].as_date_time()
    }

    pub fn last_modified(&self) -> Option<DateTime> {
        self.bson[LAST_MODIFIED].as_date_time()
    }

    pub fn unity_version(&self) -> Option<UnityVersion> {
        self.bson[UNITY_VERSION]
            .as_str()
            .and_then(UnityVersion::parse)
    }

    pub fn project_type(&self) -> ProjectType {
        self.bson[TYPE]
            .as_i32()
            .and_then(ProjectType::from_i32)
            .unwrap_or(ProjectType::Unknown)
    }

    pub fn favorite(&self) -> bool {
        self.bson[FAVORITE].as_bool().unwrap_or(false)
    }

    pub fn set_favorite(&mut self, favorite: bool) {
        self.bson.insert(FAVORITE, favorite);
    }

    pub fn set_unity_version(&mut self, unity_version: UnityVersion) {
        let version = unity_version.to_string();
        self.bson.insert(UNITY_VERSION, version.clone());
        if let Some(vrc_get) = self.bson.get_mut(VRC_GET).and_then(|x| x.as_document_mut()) {
            vrc_get.insert(CACHED_UNITY_REVISION, version);
            vrc_get.insert(UNITY_REVISION, Value::Null);
        }
    }

    pub fn set_unity_revision(
        &mut self,
        unity_version: UnityVersion,
        unity_revision: Option<String>,
    ) {
        let version = unity_version.to_string();
        self.bson.insert(UNITY_VERSION, version.clone());
        let vrc_get = self.bson.entry(VRC_GET).document_or_replace();
        vrc_get.insert(CACHED_UNITY_REVISION, version);
        vrc_get.insert(UNITY_REVISION, unity_revision);
    }

    pub fn unity_revision(&self) -> Option<&str> {
        let unity_version = self.bson[UNITY_VERSION].as_str()?;
        self.bson
            .get(VRC_GET)
            .as_document()
            .filter(|x| x[CACHED_UNITY_REVISION].as_str() == Some(unity_version))
            .and_then(|x| x[UNITY_REVISION].as_str())
    }

    pub fn custom_unity_args(&self) -> Option<Vec<String>> {
        self.bson
            .get(VRC_GET)
            .as_document()
            .and_then(|x| x[CUSTOM_UNITY_ARGS].as_array())
            .and_then(|x| {
                x.as_slice()
                    .iter()
                    .map(|x| x.as_str().map(|x| x.to_owned()))
                    .collect::<Option<Vec<_>>>()
            })
    }

    pub fn set_custom_unity_args(&mut self, custom_unity_args: Vec<String>) {
        self.bson
            .entry(VRC_GET)
            .document_or_replace()
            .insert(CUSTOM_UNITY_ARGS, Array::from(&custom_unity_args));
    }

    pub fn clear_custom_unity_args(&mut self) {
        if let Some(x) = self.bson.get_mut(VRC_GET).and_then(|x| x.as_document_mut()) {
            x.remove(CUSTOM_UNITY_ARGS);
        }
    }

    pub fn unity_path(&self) -> Option<&str> {
        self.bson[VRC_GET]
            .as_document()
            .and_then(|x| x[UNITY_PATH].as_str())
    }

    pub fn set_unity_path(&mut self, unity_path: String) {
        self.bson
            .entry(VRC_GET)
            .document_or_replace()
            .insert(UNITY_PATH, unity_path);
    }

    pub fn clear_unity_path(&mut self) {
        if let Some(x) = self.bson.get_mut(VRC_GET).and_then(|x| x.as_document_mut()) {
            x.remove(UNITY_PATH);
        }
    }
}
