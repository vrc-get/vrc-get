/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

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
use vrc_get_vpm::unity_project::UnityMigration;

/// Migrate Unity Project
#[derive(Subcommand)]
#[command(author, version)]
pub enum Migrate {
    Unity2022(Unity2022),
    #[command(name = "unity6000.0")]
    Unity6000_0(Unity6000_0),
    Vpm(Vpm),
}

multi_command!(Migrate is Unity2022, Unity6000_0, Vpm);

/// Migrate your project to Unity 2022
#[derive(Parser)]
pub struct Unity2022 {
    #[command(flatten)]
    env_args: UnityBase,
}

impl Unity2022 {
    pub async fn run(self) {
        self.env_args.run(UnityMigration::UNITY_6000_0).await;
    }
}

/// Migrate your project to Unity 6000.0
#[derive(Parser)]
pub struct Unity6000_0 {
    #[command(flatten)]
    env_args: UnityBase,
}

impl Unity6000_0 {
    pub async fn run(self) {
        self.env_args.run(UnityMigration::UNITY_6000_0).await;
    }
}

#[derive(Parser)]
pub struct UnityBase {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[cfg(not(feature = "experimental-vcc"))]
    /// Path to unity 6000.0 executable.
    #[arg(long)]
    unity: PathBuf,
    #[cfg(feature = "experimental-vcc")]
    /// Path to unity 6000.0 executable.
    #[arg(long)]
    unity: Option<PathBuf>,
    #[command(flatten)]
    env_args: EnvArgs,
}

impl UnityBase {
    pub async fn run(self, migration: &UnityMigration) {
        println!(
            "You're migrating your project to Unity {} in-place.",
            migration.unity_version_text()
        );
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
            .migrate_unity(migration, &collection, &installer)
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
                let Some(found) = connection.find_most_suitable_unity(migration.unity_version())
                else {
                    exit_with!(
                        "Unity {} not found. please load from unity hub with `vrc-get vcc unity update` or specify path with `--unity` option.",
                        migration.unity_version_text(),
                    )
                };

                if found.version() != Some(migration.unity_version()) {
                    // since we have matched with unity version, we can safely unwrap
                    log::warn!(
                        "Recommended Unity {} version is not found. Using found version: {}",
                        migration.unity_version_text(),
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
