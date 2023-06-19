use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde::Serialize;
use crate::commands::{load_env, load_unity};
use crate::version::{Version, VersionRange};

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
            requested: Vec<&'a VersionRange>,
        }

        let mut packages = vec![];

        for (package, locked) in unity.locked_packages() {
            packages.push(PackageInfo {
                name: package,
                installed: unity.get_installed_package(package).map(|x| &x.version),
                locked: Some(&locked.version),
                requested: vec![], // TODO: add requests from locked packages
            });
        }

        for (package, installed) in unity.unlocked_packages() {
            if let Some(installed) = installed {
                packages.push(PackageInfo {
                    name: package,
                    installed: Some(&installed.version),
                    locked: None,
                    requested: vec![],
                });
            }
        }

        let unlocked_names: HashSet<_> = unity
            .unlocked_packages()
            .into_iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .map(|x| x.name.as_str())
            .collect();

        let unlocked_dependencies = unity
            .unlocked_packages()
            .into_iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .flat_map(|pkg| &pkg.vpm_dependencies)
            .filter(|(k, _)| !unity.locked_packages().contains_key(k.as_str()))
            .filter(|(k, _)| !unlocked_names.contains(k.as_str()))
            .map(|(k, v)| (k, v))
            .into_group_map();
        for (package, requested) in unlocked_dependencies {
            packages.push(PackageInfo {
                name: package,
                installed: None,
                locked: None,
                requested,
            });
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
