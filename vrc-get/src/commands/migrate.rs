use crate::commands::{
    EnvArgs, ResultExt, confirm_prompt, load_collection, load_unity, update_project_last_modified,
};
use clap::{Parser, Subcommand};
use log::info;
use std::path::{Path, PathBuf};
use std::process::exit;
use tokio::process::Command;
use vrc_get_vpm::environment::PackageInstaller;
use vrc_get_vpm::io::DefaultEnvironmentIo;

/// Migrate Unity Project
#[derive(Subcommand)]
#[command(author, version)]
pub enum Migrate {
    Unity2022(Unity2022),
    Vpm(Vpm),
}

multi_command!(Migrate is Unity2022, Vpm);

/// Migrate your project to Unity 2022
#[derive(Parser)]
pub struct Unity2022 {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[cfg(not(feature = "experimental-vcc"))]
    /// Path to unity 2022 executable.
    #[arg(long)]
    unity: PathBuf,
    #[cfg(feature = "experimental-vcc")]
    /// Path to unity 2022 executable.
    #[arg(long)]
    unity: Option<PathBuf>,
    #[command(flatten)]
    env_args: EnvArgs,
}

impl Unity2022 {
    pub async fn run(self) {
        println!("You're migrating your project to Unity 2022 in-place.");
        println!("It's hard to undo this command.");
        println!("You MUST create backup of your project before running this command.");
        if !confirm_prompt("Do you want to continue?") {
            exit(1);
        }

        let mut project = load_unity(self.project).await;

        let client = crate::create_client(self.env_args.offline);
        let io = DefaultEnvironmentIo::new_default();
        let collection = load_collection(&io, client.as_ref(), self.env_args.no_update).await;
        let installer = PackageInstaller::new(&io, client.as_ref());

        #[cfg(feature = "experimental-vcc")]
        let connection = vrc_get_vpm::environment::VccDatabaseConnection::connect(&io)
            .await
            .exit_context("connecting to database");

        project
            .migrate_unity_2022(&collection, &installer)
            .await
            .exit_context("migrating unity project");

        info!(
            "Updating manifest file finished successfully. Launching Unity to finalize migration..."
        );

        #[cfg(not(feature = "experimental-vcc"))]
        let unity = self.unity;

        #[cfg(feature = "experimental-vcc")]
        let unity = match self.unity {
            Some(path) => path,
            None => {
                use vrc_get_vpm::VRCHAT_RECOMMENDED_2022_UNITY;
                let Some(found) =
                    connection.find_most_suitable_unity(VRCHAT_RECOMMENDED_2022_UNITY)
                else {
                    exit_with!(
                        "Unity 2022 not found. please load from unity hub with `vrc-get vcc unity update` or specify path with `--unity` option."
                    )
                };

                if found.version() != Some(VRCHAT_RECOMMENDED_2022_UNITY) {
                    // since we know it's unity 2022, we can safely unwrap
                    log::warn!(
                        "Recommended Unity 2022 version is not found. Using found version: {}",
                        found.version().unwrap()
                    );
                }

                PathBuf::from(found.path().unwrap())
            }
        };

        let status = Command::new(&unity)
            .args([
                "-quit".as_ref(),
                "-batchmode".as_ref(),
                "-projectPath".as_ref(),
                project.project_dir().as_os_str(),
            ])
            .status()
            .await
            .exit_context("launching unity to finalize migration");

        if !status.success() {
            exit_with!("Unity exited with status {}", status);
        }

        info!("Unity exited successfully. Migration finished.");

        update_project_last_modified(&io, project.project_dir()).await;
    }
}

/// Migrate your legacy (unitypackage) VRCSDK project to VPM project
#[derive(Parser)]
pub struct Vpm {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[command(flatten)]
    env_args: EnvArgs,
}

impl Vpm {
    pub async fn run(self) {
        println!("You're migrating your project to vpm in-place.");
        println!("It's hard to undo this command.");
        println!("You MUST create backup of your project before running this command.");
        if !confirm_prompt("Do you want to continue?") {
            exit(1);
        }

        let mut project = load_unity(self.project).await;

        let client = crate::create_client(self.env_args.offline);
        let io = DefaultEnvironmentIo::new_default();
        let collection = load_collection(&io, client.as_ref(), self.env_args.no_update).await;
        let installer = PackageInstaller::new(&io, client.as_ref());

        project
            .migrate_vpm(&collection, &installer, false)
            .await
            .exit_context("migrating unity project");

        info!("Migration finished.");

        update_project_last_modified(&io, project.project_dir()).await;
    }
}
