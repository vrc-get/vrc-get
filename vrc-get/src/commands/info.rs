use super::{UnityProject, load_collection};
use crate::commands::load_unity;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::path::Path;
use vrc_get_vpm::PackageCollection;
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::version::{UnityVersion, Version, VersionRange};

/// Shows information for other program.
#[derive(Subcommand)]
#[command(author, version)]
pub enum Info {
    Project(Project),
    Package(Package),
}

multi_command!(Info is Project, Package);

/// Show project information
///
/// Without --json-format, this will emit human readable information
/// With --json-format, this will emit machine-readable information with json
#[derive(Parser)]
#[command(author, version)]
pub struct Project {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,

    /// Output json format
    #[arg(long = "json-format")]
    json_format: Option<NonZeroU32>,
}

impl Project {
    pub async fn run(self) {
        let unity = load_unity(self.project).await;

        match self.json_format.map(|x| x.get()).unwrap_or_default() {
            0 => {
                Self::human_readable(&unity).await;
            }
            1 => {
                Self::version1(&unity).await;
            }
            unsupported => exit_with!("unsupported json version: {unsupported}"),
        };
    }

    pub async fn human_readable(unity: &UnityProject) {
        eprintln!("Project at {}", unity.project_dir().display());
        eprintln!("Using unity {}", unity.unity_version());
        eprintln!();
        eprintln!("Locked Packages:");
        for locked in unity.locked_packages() {
            if let Some(installed) = unity
                .get_installed_package(locked.name())
                .map(|x| x.version())
            {
                eprintln!(
                    "{package} version {version} with installed version {installed}",
                    package = locked.name(),
                    version = locked.version(),
                    installed = installed,
                );
            } else {
                eprintln!(
                    "{package} version {version} not installed",
                    package = locked.name(),
                    version = locked.version(),
                );
            }
        }

        eprintln!();
        eprintln!("Not Locked but installed Packages:");

        for (package, installed) in unity.unlocked_packages() {
            if let Some(installed) = installed {
                eprintln!(
                    "{package} version {installed}",
                    installed = installed.version()
                );
            }
        }
    }

    pub async fn version1(unity: &UnityProject) {
        #[derive(Serialize)]
        struct Project<'a> {
            unity_version: Option<UnityVersion>,
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

        for locked in unity.locked_packages() {
            packages.push(PackageInfo {
                name: locked.name(),
                installed: unity
                    .get_installed_package(locked.name())
                    .map(|x| x.version()),
                locked: Some(locked.version()),
                requested: vec![], // TODO: add requests from locked packages
            });
        }

        for (package, installed) in unity.unlocked_packages() {
            if let Some(installed) = installed {
                packages.push(PackageInfo {
                    name: package,
                    installed: Some(installed.version()),
                    locked: None,
                    requested: vec![],
                });
            }
        }

        let unlocked_names: HashSet<_> = unity
            .unlocked_packages()
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .map(|x| x.name())
            .collect();

        let unlocked_dependencies = unity
            .unlocked_packages()
            .iter()
            .filter_map(|(_, pkg)| pkg.as_ref())
            .flat_map(|pkg| pkg.vpm_dependencies())
            .filter(|(k, _)| !unity.is_locked(k.as_ref()))
            .filter(|(k, _)| !unlocked_names.contains(k.as_ref()))
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
            unity_version: Some(unity.unity_version()),
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
        let client = crate::create_client(self.env_args.offline);
        let io = DefaultEnvironmentIo::new_default();
        let collection = load_collection(&io, client.as_ref(), self.env_args.no_update).await;

        let format_version = match self.json_format.map(|x| x.get()).unwrap_or_default() {
            0 => {
                eprintln!("warning: no --json-format is specified! using lastest version 1");
                1
            }
            supported @ 1..=1 => supported,
            unsupported => exit_with!("unsupported json version: {unsupported}"),
        };

        debug_assert_eq!(format_version, 1);

        let versions: Vec<_> = collection
            .find_packages(&self.package)
            .map(|x| PackageVersionInfo {
                version: x.version(),
                // since 1.5.0
                is_yanked: x.is_yanked(),
            })
            .collect();

        #[derive(Serialize)]
        struct PackageInfo<'a> {
            versions: &'a [PackageVersionInfo<'a>],
        }

        #[derive(Serialize)]
        struct PackageVersionInfo<'a> {
            version: &'a Version,
            is_yanked: bool,
        }

        let package_info = PackageInfo {
            versions: versions.as_slice(),
        };

        println!("{}", serde_json::to_string(&package_info).unwrap());
    }
}
