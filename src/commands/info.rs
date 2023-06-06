use std::num::NonZeroU32;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use serde::Serialize;
use crate::commands::{load_env, load_unity};
use crate::version::Version;

/// Shows information for other program.
#[derive(Subcommand)]
#[command(author, version)]
pub enum Info {
    Project(Project),
    Package(Package),
}

multi_command!(Info is Project, Package);

/// Show project information
#[derive(Parser)]
#[command(author, version)]
pub struct Project {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,

    /// Output json format
    #[arg(long = "json-format")]
    json_format: Option<NonZeroU32>,
}

impl Project {
    pub async fn run(self) {
        let unity = load_unity(self.project).await;

        let format_version = match self.json_format.map(|x| x.get()).unwrap_or_default() {
            0 => {
                eprintln!("warning: no --json-format is specified! using lastest version 1");
                1
            }
            supported @ 1..=1 => supported,
            unsupported => exit_with!("unsupported json version: {unsupported}"),
        };

        debug_assert_eq!(format_version, 1);

        #[derive(Serialize)]
        struct Project<'a> {
            packages: &'a [PackageInfo<'a>],
        }

        #[derive(Serialize)]
        struct PackageInfo<'a> {
            name: &'a str,
            installed: Option<&'a Version>,
            locked: Option<&'a Version>,
        }

        let mut packages = vec![];

        for (package, locked) in unity.locked_packages() {
            packages.push(PackageInfo {
                name: package,
                installed: unity.get_installed_package(package).map(|x| &x.version),
                locked: Some(&locked.version),
            });
        }

        for (package, installed) in unity.unlocked_packages() {
            if let Some(installed) = installed {
                packages.push(PackageInfo {
                    name: package,
                    installed: Some(&installed.version),
                    locked: None,
                });
            }
        }

        let project = Project {
            packages: packages.as_slice(),
        };

        println!("{}", serde_json::to_string(&project).unwrap());
    }
}

/// Show project information
#[derive(Parser)]
#[command(author, version)]
pub struct Package {
    #[arg()]
    package: String,
    #[command(flatten)]
    env_args: super::EnvArgs,

    /// Output json format
    #[arg(long = "json-format")]
    json_format: Option<NonZeroU32>,
}

impl Package {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;

        let format_version = match self.json_format.map(|x| x.get()).unwrap_or_default() {
            0 => {
                eprintln!("warning: no --json-format is specified! using lastest version 1");
                1
            }
            supported @ 1..=1 => supported,
            unsupported => exit_with!("unsupported json version: {unsupported}"),
        };

        debug_assert_eq!(format_version, 1);

        let packages = env.find_packages(&self.package);

        let versions: Vec<_> = packages.iter().map(|x| PackageVersionInfo {version: x.version()}).collect();

        #[derive(Serialize)]
        struct PackageInfo<'a> {
            versions: &'a [PackageVersionInfo<'a>]
        }

        #[derive(Serialize)]
        struct PackageVersionInfo<'a> {
            version: &'a Version
        }

        let package_info = PackageInfo {
            versions: versions.as_slice()
        };

        println!("{}", serde_json::to_string(&package_info).unwrap());
    }
}
