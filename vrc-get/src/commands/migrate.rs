use crate::commands::{confirm_prompt, load_env, load_unity, EnvArgs, ResultExt};
use clap::{Parser, Subcommand};
use log::warn;
use std::path::PathBuf;
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
    project: Option<PathBuf>,
    /// Path to unity 2022 executable.
    #[arg(long)]
    unity: PathBuf,
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
            .migrate_unity_2022(&env, &self.unity)
            .await
            .exit_context("migrating unity project");

        // Already saved in migrate_unity_2022
    }
}
