use crate::vpm::VersionSelector;
use clap::{Parser, Subcommand};
use semver::Version;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum Command {
    Package(Package),
}

impl Command {
    pub async fn run(self) {
        match self {
            Command::Package(cmd) => cmd.run().await,
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
    project: Option<String>,
}

impl Package {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");
        let mut unity =
            crate::vpm::UnityProject::find_unity_project(self.project.map(PathBuf::from))
                .await
                .expect("unity project not found");

        let version_selector = match self.version {
            None => VersionSelector::Latest,
            Some(ref version) => VersionSelector::Specific(version),
        };
        let package = unity
            .find_package_by_name(&mut env, &self.name, version_selector)
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
