use crate::commands::{load_env, ResultExt};
use clap::{Parser, Subcommand};
use log::warn;
use std::cmp::Reverse;
use std::path::Path;
use vrc_get_vpm::io::DefaultProjectIo;
use vrc_get_vpm::{unity_hub, UnityProject};

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

/// List projects
#[derive(Parser)]
#[command(author, version)]
pub struct ProjectList {
    #[command(flatten)]
    env_args: super::EnvArgs,
}

impl ProjectList {
    pub async fn run(self) {
        let mut env = load_env(&self.env_args).await;

        env.migrate_from_settings_json()
            .await
            .exit_context("migrating from settings.json");

        env.sync_with_real_projects(false)
            .await
            .exit_context("syncing with real projects");

        let mut projects = env.get_projects().exit_context("getting projects");

        projects.sort_by_key(|x| Reverse(x.last_modified().timestamp_millis()));

        for project in projects.iter() {
            let path = project.path();
            // TODO: use '/' for unix
            let name = project.name();
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
        let mut env = load_env(&self.env_args).await;

        let project =
            UnityProject::load(DefaultProjectIo::new(Path::new(self.path.as_ref()).into()))
                .await
                .exit_context("loading specified project");

        if !project.is_valid().await {
            return eprintln!("Invalid project at {}", self.path);
        }

        env.migrate_from_settings_json()
            .await
            .exit_context("migrating from settings.json");

        env.add_project(&project)
            .await
            .exit_context("adding project");
        env.save().await.exit_context("saving environment");
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
        let mut env = load_env(&self.env_args).await;

        let Some(project) = env
            .get_projects()
            .exit_context("getting projects")
            .into_iter()
            .find(|x| x.path() == self.path.as_ref())
        else {
            return println!("No project found at {}", self.path);
        };

        env.migrate_from_settings_json()
            .await
            .exit_context("migrating from settings.json");

        env.remove_project(&project)
            .exit_context("removing project");
        env.save().await.exit_context("saving environment");
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
        let env = load_env(&self.env_args).await;

        let mut unity_installations = env
            .get_unity_installations()
            .exit_context("getting installations");

        unity_installations.sort_by_key(|x| Reverse(x.version()));

        for unity in unity_installations.iter() {
            if let Some(unity_version) = unity.version() {
                println!("version {} at {}", unity_version, unity.path());
            } else {
                println!("unknown version at {}", unity.path());
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
        let mut env = load_env(&self.env_args).await;

        let unity_version = vrc_get_vpm::unity::call_unity_for_version(self.path.as_ref().as_ref())
            .await
            .exit_context("calling unity for version");

        env.add_unity_installation(self.path.as_ref(), unity_version)
            .await
            .exit_context("adding unity installation");

        println!("Added version {} at {}", unity_version, self.path);

        env.save().await.exit_context("saving environment");
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
        let mut env = load_env(&self.env_args).await;

        let Some(unity) = env
            .get_unity_installations()
            .exit_context("getting installations")
            .into_iter()
            .find(|x| x.path() == self.path.as_ref())
        else {
            return eprintln!("No unity installation found at {}", self.path);
        };

        env.remove_unity_installation(&unity)
            .await
            .exit_context("adding unity installation");

        env.save().await.exit_context("saving environment");
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
}

impl UnityUpdate {
    pub async fn run(self) {
        let mut env = load_env(&self.env_args).await;

        let unity_hub_path = env
            .find_unity_hub()
            .await
            .exit_context("loading unity hub path")
            .unwrap_or_else(|| exit_with!("Unity Hub not found"));

        let paths_from_hub = unity_hub::get_unity_from_unity_hub(unity_hub_path.as_ref())
            .await
            .exit_context("loading unity list from unity hub");

        env.update_unity_from_unity_hub_and_fs(&paths_from_hub)
            .await
            .exit_context("updating unity from unity hub");

        env.save().await.exit_context("saving environment");
    }
}
