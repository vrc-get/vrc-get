use crate::version::Version;
use crate::vpm::VersionSelector;
use clap::{Parser, Subcommand};
use reqwest::Url;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    Package(Package),
    Repo(Repo),
}

impl Command {
    pub async fn run(self) {
        match self {
            Command::Package(cmd) => cmd.run().await,
            Command::Repo(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser)]
pub struct Package {
    /// Name of Package
    #[arg()]
    name: String,
    #[arg()]
    version: Option<Version>,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
}

impl Package {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");
        let mut unity = crate::vpm::UnityProject::find_unity_project(self.project)
            .await
            .expect("unity project not found");

        let version_selector = match self.version {
            None => VersionSelector::Latest,
            Some(ref version) => VersionSelector::Specific(version),
        };
        let package = env
            .find_package_by_name(&self.name, version_selector)
            .await
            .expect("finding package")
            .expect("no matching package not found");
        unity
            .add_package(&mut env, &package)
            .await
            .expect("adding package");

        unity.save().await.expect("saving manifest file");
    }
}

#[derive(Parser)]
pub struct Repo {
    /// Name of Package
    #[arg()]
    path_or_url: String,
}

impl Repo {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        if let Ok(url) = Url::parse(&self.path_or_url) {
            env.add_remote_repo(url).await.expect("adding repository")
        } else {
            env.add_local_repo(Path::new(&self.path_or_url))
                .await
                .expect("adding repository")
        }

        env.save().await.expect("saving settings file");
    }
}
