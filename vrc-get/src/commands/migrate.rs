use crate::commands::{confirm_prompt, load_env, load_unity, EnvArgs, ResultExt};
use clap::{Parser, Subcommand};
use log::{info, warn};
use std::path::{Path, PathBuf};
use std::process::exit;

/// Migrate Unity Project
#[derive(Subcommand)]
#[command(author, version)]
pub enum Migrate {
    #[command(subcommand)]
    Unity(Unity),
}

multi_command!(Migrate is Unity);

#[derive(Subcommand)]
#[command(author, version)]
pub enum Unity {
    #[command(name = "2022")]
    Unity2022(Unity2022),
}

multi_command!(Unity is Unity2022);

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
        warn!("migrate unity-to-2022 is unstable command.");
        println!("You're migrating your project to Unity 2022 in-place.");
        println!("It's hard to undo this command.");
        println!("You MUST create backup of your project before running this command.");
        if !confirm_prompt("Do you want to continue?") {
            exit(1);
        }

        let mut project = load_unity(self.project).await;
        let env = load_env(&self.env_args).await;

        project
            .migrate_unity_2022(&env)
            .await
            .exit_context("migrating unity project");

        project.save().await.exit_context("saving project");

        info!("Updating manifest file finished successfully. Launching Unity to finalize migration...");

        #[cfg(not(feature = "experimental-vcc"))]
        let unity = self.unity;

        #[cfg(feature = "experimental-vcc")]
        let unity = self.unity.unwrap_or_else(|| {
            use vrc_get_vpm::version::ReleaseType;
            use vrc_get_vpm::version::UnityVersion;
            let recommended = UnityVersion::new(2022, 3, 6, ReleaseType::Normal, 1);
            let Some(found) = env.find_most_suitable_unity(recommended)
                .exit_context("getting unity 2022 path") else {
                exit_with!("Unity 2022 not found. please load from unity hub with `vrc-get vcc unity update` or specify path with `--unity` option.")
            };

            if found.version() != Some(recommended) {
                // since we know it's unity 2022, we can safely unwrap
                warn!("Recommended Unity 2022 version is not found. Using found version: {}", found.version().unwrap());
            }

            PathBuf::from(found.path())
        });

        project
            .call_unity(&unity)
            .await
            .exit_context("launching unity to finalize migration");

        info!("Unity exited successfully. Migration finished.")
    }
}
