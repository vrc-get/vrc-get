use crate::commands::{ResultExt, absolute_path};
use clap::{Parser, Subcommand, ValueEnum};
use log::warn;
use std::cmp::Reverse;
use std::fmt::{Display, Formatter};
use std::path::Path;
use vrc_get_vpm::environment::{Settings, VccDatabaseConnection, find_unity_hub};
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo};
use vrc_get_vpm::{UnityProject, unity_hub};

/// Experimental VCC commands
#[derive(Subcommand)]
#[command(author, version)]
pub enum Vcc {
    #[command(subcommand)]
    Project(Project),
    #[command(subcommand)]
    Unity(Unity),
}

impl Vcc {
    pub async fn run(self) {
        warn!("vrc-get vcc is experimental and may change in the future!");
        self.run_inner().await;
    }
}

multi_command!(fn run_inner Vcc is Project, Unity);

/// Vcc Project Commands
#[derive(Subcommand)]
#[command(author, version)]
pub enum Project {
    List(ProjectList),
    Add(ProjectAdd),
    Remove(ProjectRemove),
}

multi_command!(Project is List, Add, Remove);

async fn migrate_sanitize_projects(
    connection: &mut VccDatabaseConnection,
    io: &DefaultEnvironmentIo,
    settings: &Settings,
) {
    // migrate from settings json
    connection
        .migrate(settings, io)
        .await
        .exit_context("migrating from settings.json");
    connection.dedup_projects();
}

/// List projects
#[derive(Parser)]
#[command(author, version)]
pub struct ProjectList {
    #[command(flatten)]
    env_args: super::EnvArgs,
}

impl ProjectList {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let settings = Settings::load(&io).await.exit_context("loading settings");

        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        migrate_sanitize_projects(&mut connection, &io, &settings).await;

        connection
            .sync_with_real_projects(false, &io)
            .await
            .exit_context("syncing with real projects");

        let mut projects = connection.get_projects();

        connection
            .save(&io)
            .await
            .exit_context("saving updated database");

        projects.sort_by_key(|x| Reverse(x.last_modified()));

        for project in projects.iter() {
            let Some(path) = project.path() else { continue };
            let Some(name) = project.name() else { continue };
            let unity_version = project
                .unity_version()
                .map(|x| x.to_string())
                .unwrap_or("unknown".into());

            println!("{name}:");
            println!("  Path: {}", path);
            println!("  Unity: {unity_version}");
            println!("  Target: {}", project.project_type());
            println!("  Is Favorite: {}", project.favorite());
        }
    }
}

/// Add Project to vpm project management
#[derive(Parser)]
#[command(author, version)]
pub struct ProjectAdd {
    #[command(flatten)]
    env_args: super::EnvArgs,
    path: Box<str>,
}

impl ProjectAdd {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let mut settings = Settings::load(&io).await.exit_context("loading settings");
        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        let project_path = absolute_path(Path::new(self.path.as_ref()));
        let project_io = DefaultProjectIo::new(project_path.into());
        let project = UnityProject::load(project_io)
            .await
            .exit_context("loading specified project");

        migrate_sanitize_projects(&mut connection, &io, &settings).await;

        connection
            .add_project(&project)
            .await
            .exit_context("adding project");

        connection.save(&io).await.exit_context("saving database");
        settings
            .load_from_db(&connection)
            .exit_context("saving database");
        settings.save(&io).await.exit_context("saving settings");
    }
}

/// Remove Project from vpm project management
#[derive(Parser)]
#[command(author, version)]
pub struct ProjectRemove {
    #[command(flatten)]
    env_args: super::EnvArgs,
    path: Box<str>,
}

impl ProjectRemove {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let mut settings = Settings::load(&io).await.exit_context("loading settings");
        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        let Some(project) = connection
            .find_project(self.path.as_ref())
            .exit_context("getting projects")
        else {
            return println!("No project found at {}", self.path);
        };

        migrate_sanitize_projects(&mut connection, &io, &settings).await;

        connection.remove_project(&project);

        connection.save(&io).await.exit_context("saving database");
        settings
            .load_from_db(&connection)
            .exit_context("saving database");
        settings.save(&io).await.exit_context("saving environment");
    }
}

/// Vcc Unity Management Commands
#[derive(Subcommand)]
#[command(author, version)]
pub enum Unity {
    List(UnityList),
    Add(UnityAdd),
    Remove(UnityRemove),
    Update(UnityUpdate),
}

multi_command!(Unity is List, Add, Remove, Update);

/// List registered Unity installations
#[derive(Parser)]
#[command(author, version)]
pub struct UnityList {
    #[command(flatten)]
    env_args: super::EnvArgs,
}

impl UnityList {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        let mut unity_installations = connection.get_unity_installations();

        unity_installations.sort_by_key(|x| Reverse(x.version()));

        for unity in unity_installations.iter() {
            if let Some(path) = unity.path() {
                if let Some(unity_version) = unity.version() {
                    println!("version {} at {}", unity_version, path);
                } else {
                    println!("unknown version at {}", path);
                }
            }
        }
    }
}

/// Add Unity installation to the list
#[derive(Parser)]
#[command(author, version)]
pub struct UnityAdd {
    #[command(flatten)]
    env_args: super::EnvArgs,
    path: Box<str>,
}

impl UnityAdd {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        let unity_version = vrc_get_vpm::unity::call_unity_for_version(self.path.as_ref().as_ref())
            .await
            .exit_context("calling unity for version");

        connection
            .add_unity_installation(self.path.as_ref(), unity_version)
            .exit_context("adding unity installation");

        connection.save(&io).await.exit_context("saving database");

        println!("Added version {} at {}", unity_version, self.path);
    }
}

/// Remove specified Unity installation from the list
#[derive(Parser)]
#[command(author, version)]
pub struct UnityRemove {
    #[command(flatten)]
    env_args: super::EnvArgs,
    path: Box<str>,
}

impl UnityRemove {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        let Some(unity) = connection
            .get_unity_installations()
            .into_iter()
            .find(|x| x.path() == Some(self.path.as_ref()))
        else {
            return eprintln!("No unity installation found at {}", self.path);
        };

        connection.remove_unity_installation(&unity);

        connection.save(&io).await.exit_context("saving database");
    }
}

/// Update Unity installation list from file system and Unity Hub.
///
/// If the installation is not found in the file system, it will be removed from the list.
/// If the installation is found from Unity Hub, it will be added to the list.
#[derive(Parser)]
#[command(author, version)]
pub struct UnityUpdate {
    #[command(flatten)]
    env_args: super::EnvArgs,
    /// The method to get the list of Unity from Unity Hub.
    #[arg(long, default_value_t)]
    method: UnityHubAccessMethod,
}

#[derive(Default, Copy, Clone, Eq, Ord, PartialOrd, PartialEq, ValueEnum)]
enum UnityHubAccessMethod {
    /// Reads config files of Unity Hub
    #[default]
    ReadConfig,
    /// Launches headless Unity Hub in background
    CallHub,
}

impl Display for UnityHubAccessMethod {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnityHubAccessMethod::ReadConfig => f.write_str("read-config"),
            UnityHubAccessMethod::CallHub => f.write_str("call-hub"),
        }
    }
}

impl UnityUpdate {
    pub async fn run(self) {
        let io = DefaultEnvironmentIo::new_default();
        let mut settings = Settings::load(&io).await.exit_context("loading settings");

        let unity_hub_path = find_unity_hub(&mut settings, &io)
            .await
            .exit_context("loading unity hub path")
            .unwrap_or_else(|| exit_with!("Unity Hub not found"));

        let unity_list = match self.method {
            UnityHubAccessMethod::ReadConfig => unity_hub::load_unity_by_loading_unity_hub_files()
                .await
                .exit_context("loading list of unity from config file")
                .into_iter()
                .map(|x| (x.version, x.path))
                .collect::<Vec<_>>(),
            UnityHubAccessMethod::CallHub => {
                unity_hub::load_unity_by_calling_unity_hub(unity_hub_path.as_ref())
                    .await
                    .exit_context("loading unity list from unity hub")
            }
        };

        let mut connection = VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");
        connection
            .update_unity_from_unity_hub_and_fs(&unity_list, &io)
            .await
            .exit_context("updating unity from unity hub");

        connection.save(&io).await.exit_context("saving database");
        settings.save(&io).await.exit_context("saving settings");
    }
}
