use crate::version::Version;
use crate::vpm::structs::package::PackageJson;
use crate::vpm::structs::remote_repo::PackageVersions;
use crate::vpm::{
    download_remote_repository, AddPackageErr, Environment, UnityProject, VersionSelector,
};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde::Serialize;
use serde_json::{from_value, Map, Value};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::exit;
use tokio::fs::{read_dir, remove_file};

macro_rules! multi_command {
    ($class: ident is $($variant: ident),*) => {
        impl $class {
            pub async fn run(self) {
                match self {
                    $($class::$variant(cmd) => cmd.run().await,)*
                }
            }
        }
    };
}

// small wrapper utilities

macro_rules! exit_with {
    ($($tt:tt)*) => {{
        eprintln!($($tt)*);
        exit(1)
    }};
}

async fn load_env(client: Option<reqwest::Client>) -> Environment {
    Environment::load_default(client)
        .await
        .exit_context("loading global config")
}

async fn load_unity(path: Option<PathBuf>) -> UnityProject {
    UnityProject::find_unity_project(path)
        .await
        .exit_context("loading unity project")
}

async fn get_package<'a>(
    env: &'a Environment,
    name: &str,
    version_selector: VersionSelector<'a>,
) -> PackageJson {
    env.find_package_by_name(&name, version_selector)
        .await
        .exit_context("finding package")
        .unwrap_or_else(|| exit_with!("no matching package not found"))
}

async fn mark_and_sweep(unity: &mut UnityProject) {
    for x in unity
        .mark_and_sweep()
        .await
        .exit_context("sweeping unused packages")
    {
        eprintln!("removed {x} which is unused");
    }
}

async fn save_unity(unity: &mut UnityProject) {
    unity.save().await.exit_context("saving manifest file");
}

async fn save_env(env: &mut Environment) {
    env.save().await.exit_context("saving global config");
}

trait ResultExt<T, E>: Sized {
    fn exit_context(self, context: &str) -> T
    where
        E: Display;
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn exit_context(self, context: &str) -> T
    where
        E: Display,
    {
        match self {
            Ok(value) => value,
            Err(err) => exit_with!("error {context}: {err}"),
        }
    }
}

/// Open Source command line interface of VRChat Package Manager.
#[derive(Parser)]
#[command(author, version, about)]
pub enum Command {
    #[command(alias = "i", alias = "resolve")]
    Install(Install),
    #[command(alias = "rm")]
    Remove(Remove),
    Outdated(Outdated),
    Upgrade(Upgrade),
    Search(Search),
    #[command(subcommand)]
    Repo(Repo),
}

multi_command!(Command is Install, Remove, Outdated, Upgrade, Search, Repo);

/// Adds package to unity project
///
/// With install command, you'll add to dependencies. With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version)]
pub struct Install {
    /// Name of Package
    #[arg()]
    name: Option<String>,
    /// Version of package. if not specified, latest version will be used
    #[arg(id = "VERSION")]
    version: Option<Version>,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl Install {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;
        let mut unity = load_unity(self.project).await;

        if let Some(name) = self.name {
            let version_selector = match self.version {
                None if self.prerelease => VersionSelector::LatestIncluidingPrerelease,
                None => VersionSelector::Latest,
                Some(ref version) => VersionSelector::Specific(version),
            };
            let package = get_package(&env, &name, version_selector).await;
            unity
                .add_package(&env, &package)
                .await
                .exit_context("adding package");

            mark_and_sweep(&mut unity).await;
        } else {
            unity.resolve(&env).await.exit_context("resolving packages");
        }

        unity.save().await.exit_context("saving manifest file");
    }
}

/// Remove package from Unity project.
#[derive(Parser)]
#[command(author, version)]
pub struct Remove {
    /// Name of Packages to remove
    #[arg()]
    names: Vec<String>,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
}

impl Remove {
    pub async fn run(self) {
        let mut unity = load_unity(self.project).await;

        unity
            .remove(&self.names.iter().map(String::as_ref).collect::<Vec<_>>())
            .await
            .exit_context("removing package");

        mark_and_sweep(&mut unity).await;

        save_unity(&mut unity).await;
    }
}

/// Show list of outdated packages
#[derive(Parser)]
#[command(author, version)]
pub struct Outdated {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,

    /// With this option, output is printed in json format
    #[arg(long = "json-format")]
    json_format: Option<NonZeroU32>,

    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl Outdated {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;
        let unity = load_unity(self.project).await;

        let mut outdated_packages = HashMap::new();

        for (name, dep) in unity.locked_packages() {
            match env
                .find_package_by_name(name, VersionSelector::Latest)
                .await
            {
                Err(e) => log::error!("error loading package {}: {}", name, e),
                Ok(None) => log::error!("package {} not found.", name),
                // if found version is newer: add to outdated
                Ok(Some(pkg)) if dep.version < pkg.version => {
                    outdated_packages.insert(pkg.name.clone(), (pkg, &dep.version));
                }
                Ok(Some(_)) => (),
            }
        }

        for dep in unity.locked_packages().values() {
            for (name, range) in &dep.dependencies {
                if let Some((outdated, _)) = outdated_packages.get(name) {
                    if !range.matches(&outdated.version) {
                        outdated_packages.remove(name);
                    }
                }
            }
        }

        match self.json_format.map(|x| x.get()).unwrap_or(0) {
            0 => {
                for (name, (found, installed)) in &outdated_packages {
                    println!(
                        "{}: installed: {}, found: {}",
                        name, installed, &found.version
                    );
                }
            }
            1 => {
                #[derive(Serialize)]
                struct OutdatedInfo {
                    package_name: String,
                    installed_version: Version,
                    newer_version: Version,
                }
                let info = outdated_packages
                    .into_iter()
                    .map(|(package_name, (found, installed))| OutdatedInfo {
                        package_name,
                        installed_version: installed.clone(),
                        newer_version: found.version,
                    })
                    .collect::<Vec<_>>();
                println!("{}", serde_json::to_string(&info).unwrap());
            }
            v => exit_with!("unsupported json version: {v}"),
        }
    }
}

/// Upgrade specified package or all packages to latest or specified version.
///
/// With install command, you'll add to dependencies. With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version)]
pub struct Upgrade {
    /// Name of Package
    #[arg()]
    name: Option<String>,
    /// Version of package. if not specified, latest version will be used
    #[arg(id = "VERSION")]
    version: Option<Version>,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl Upgrade {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;
        let mut unity = load_unity(self.project).await;

        if let Some(name) = self.name {
            let version_selector = match self.version {
                None if self.prerelease => VersionSelector::LatestIncluidingPrerelease,
                None => VersionSelector::Latest,
                Some(ref version) => VersionSelector::Specific(version),
            };
            let package = get_package(&env, &name, version_selector).await;

            unity
                .upgrade_package(&env, &package)
                .await
                .exit_context("upgrading package");

            println!("upgraded {} to {}", name, package.version);
        } else {
            let version_selector = match self.prerelease {
                true => VersionSelector::LatestIncluidingPrerelease,
                false => VersionSelector::Latest,
            };
            let package_names = unity.locked_packages().keys().cloned().collect::<Vec<_>>();
            for name in package_names {
                let package = get_package(&env, &name, version_selector).await;

                match unity.upgrade_package(&env, &package).await {
                    Ok(_) => {
                        println!("upgraded {} to {}", name, package.version);
                    }
                    Err(AddPackageErr::Io(e)) => log::error!("upgrading package: {}", e),
                    Err(AddPackageErr::AlreadyNewerPackageInstalled) => {}
                    Err(e) => {
                        log::warn!("upgrading {} to {}: {}", name, package.version, e);
                    }
                }
            }
        }

        mark_and_sweep(&mut unity).await;
        save_unity(&mut unity).await;
    }
}

/// Search package by the query
///
/// Search for packages that includes query in either name, displayName, or description.
#[derive(Parser)]
#[command(author, version)]
pub struct Search {
    /// Name of Package
    #[arg(required = true, name = "QUERY")]
    queries: Vec<String>,

    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl Search {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;

        let mut queries = self.queries;
        for query in &mut queries {
            query.make_ascii_lowercase();
        }

        fn search_targets(pkg: &PackageJson) -> Vec<String> {
            let mut sources = Vec::with_capacity(3);

            sources.push(pkg.name.as_str().to_ascii_lowercase());
            sources.extend(pkg.display_name.as_deref().map(|x| x.to_ascii_lowercase()));
            sources.extend(pkg.description.as_deref().map(|x| x.to_ascii_lowercase()));

            sources
        }

        let found_packages = env
            .find_whole_all_packages(|pkg| {
                // filtering
                let search_targets = search_targets(pkg);

                queries
                    .iter()
                    .all(|query| search_targets.iter().any(|x| x.contains(query)))
            })
            .await
            .exit_context("searching whole repositories");

        if found_packages.is_empty() {
            println!("No matching package found!")
        } else {
            for x in found_packages {
                if let Some(name) = x.display_name {
                    println!("{} version {}", name, x.version);
                    println!("({})", x.name);
                } else {
                    println!("{} version {}", x.name, x.version);
                }
                if let Some(description) = x.description {
                    println!("{}", description);
                }
                println!();
            }
        }
    }
}

/// Commands around repositories
#[derive(Subcommand)]
#[command(author, version)]
pub enum Repo {
    List(RepoList),
    Add(RepoAdd),
    Remove(RepoRemove),
    Cleanup(RepoCleanup),
    Packages(RepoPackages),
}

multi_command!(Repo is List, Add, Remove, Cleanup, Packages);

/// List all repositories
#[derive(Parser)]
#[command(author, version)]
pub struct RepoList {
    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl RepoList {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;

        for repo in env.get_repos().await.exit_context("getting all repos") {
            let mut name = None;
            let mut r#type = None;
            let mut local_path = None;
            if let Some(description) = &repo.description {
                name = name.or(description.name.as_deref());
                r#type = r#type.or(description.r#type.as_deref());
            }
            if let Some(creation_info) = &repo.creation_info {
                name = name.or(creation_info.name.as_deref());
                local_path = local_path.or(creation_info.local_path.as_deref());
            }
            println!(
                "{} | {} (at {})",
                name.unwrap_or("(unnamed)"),
                r#type.unwrap_or("(unknown type)"),
                local_path.unwrap_or(Path::new("(unknown)")).display(),
            );
        }
    }
}

/// Add remote or local repository
#[derive(Parser)]
#[command(author, version)]
pub struct RepoAdd {
    /// URL of Package
    #[arg()]
    path_or_url: String,
    /// Name of Package
    #[arg()]
    name: Option<String>,

    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl RepoAdd {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let mut env = load_env(client).await;

        if let Ok(url) = Url::parse(&self.path_or_url) {
            env.add_remote_repo(url, self.name.as_deref())
                .await
                .exit_context("adding repository")
        } else {
            env.add_local_repo(Path::new(&self.path_or_url), self.name.as_deref())
                .await
                .exit_context("adding repository")
        }

        save_env(&mut env).await;
    }
}
/// Remove repository with specified url, path or name
#[derive(Parser)]
#[command(author, version)]
pub struct RepoRemove {
    /// URL of Package
    #[arg()]
    name_or_url: String,

    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl RepoRemove {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let mut env = load_env(client).await;

        let removed = if let Ok(url) = Url::parse(&self.name_or_url) {
            env.remove_repo(|x| x.url.as_deref() == Some(url.as_str()))
                .await
                .exit_context("removing based on url")
        } else {
            let path = Path::new(&self.name_or_url);
            env.remove_repo(|x| x.local_path.as_path() == path)
                .await
                .exit_context("removing based on path")
        };

        if !removed {
            env.remove_repo(|x| x.name.as_deref() == Some(self.name_or_url.as_str()))
                .await
                .exit_context("removing based on name");
        }

        save_env(&mut env).await;
    }
}

/// Cleanup repositories in Repos directory
///
/// The official VPM CLI will add <uuid>.json in the Repos directory even if error occurs.
/// So this command will cleanup Repos directory.
#[derive(Parser)]
#[command(author, version)]
pub struct RepoCleanup {
    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl RepoCleanup {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let env = load_env(client).await;

        let mut uesr_repo_file_names = vec![
            OsString::from("vrc-official.json"),
            OsString::from("vrc-curated.json"),
        ];
        let repos_base = env.get_repos_dir();

        for x in env.get_user_repos().exit_context("reading user repos") {
            if let Ok(relative) = x.local_path.strip_prefix(&repos_base) {
                if let Some(file_name) = relative.file_name() {
                    if relative
                        .parent()
                        .map(|x| x.as_os_str().is_empty())
                        .unwrap_or(true)
                    {
                        // the file must be in direct child of
                        uesr_repo_file_names.push(file_name.to_owned());
                    }
                }
            }
        }

        let mut entry = read_dir(repos_base).await.exit_context("reading dir");
        while let Some(entry) = entry.next_entry().await.exit_context("reading dir") {
            let path = entry.path();
            if tokio::fs::metadata(&path)
                .await
                .map(|x| x.is_file())
                .unwrap_or(false)
                && path.extension() == Some(OsStr::new("json"))
                && !uesr_repo_file_names.contains(&entry.file_name())
            {
                remove_file(path)
                    .await
                    .exit_context("removing unused files");
            }
        }
    }
}

/// Remove repository from user repositories.
#[derive(Parser)]
#[command(author, version)]
pub struct RepoPackages {
    name_or_url: String,

    /// do not connect to remote servers, use local caches only
    #[arg(long)]
    offline: bool,
}

impl RepoPackages {
    pub async fn run(self) {
        fn print_repo(cache: Map<String, Value>) {
            for (package, value) in cache {
                let versions =
                    from_value::<PackageVersions>(value).exit_context("loading package data");
                if let Some((_, pkg)) = versions.versions.first() {
                    if let Some(display_name) = &pkg.display_name {
                        println!("{} | {}", display_name, package);
                    } else {
                        println!("{}", package);
                    }
                    if let Some(description) = &pkg.description {
                        println!("{}", description);
                    }
                    for (version, pkg) in &versions.versions {
                        println!("{}: {}", version, pkg.url);
                    }
                    println!();
                }
            }
        }

        let client = crate::create_client(self.offline);

        if let Some(url) = Url::parse(&self.name_or_url).ok() {
            let Some(client) = client else {
                exit_with!("remote repository specified but offline mode.");
            };
            let repo = download_remote_repository(&client, url, None)
                .await
                .exit_context("downloading repository")
                .unwrap()
                .0;

            let cache = repo
                .get("packages")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or(Map::<String, Value>::new());

            print_repo(cache);
        } else {
            let env = load_env(client).await;
            let some_name = Some(self.name_or_url.as_str());
            let mut found = false;

            for repo in env.get_repos().await.exit_context("loading repos") {
                if repo.creation_info.as_ref().and_then(|x| x.name.as_deref()) == some_name
                    || repo.description.as_ref().and_then(|x| x.name.as_deref()) == some_name
                {
                    print_repo(repo.cache.clone());
                    found = true;
                }
            }

            if !found {
                exit_with!("no repository named {} found!", self.name_or_url);
            }
        }
    }
}
