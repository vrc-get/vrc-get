use crate::version::Version;
use crate::vpm::structs::package::PackageJson;
use crate::vpm::structs::repository::Repository;
use crate::vpm::{AddPackageRequest, download_remote_repository, Environment, PackageInfo, UnityProject, VersionSelector};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde::Serialize;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::exit;
use dialoguer::Confirm;
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

fn get_package<'env>(
    env: &'env Environment,
    name: &str,
    version_selector: VersionSelector,
) -> PackageInfo<'env> {
    env.find_package_by_name(&name, version_selector)
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

fn confirm_prompt(msg: &str) -> bool {
    Confirm::new().with_prompt(msg).interact().unwrap_or(false)
}

fn print_prompt_install(request: &AddPackageRequest, yes: bool) {
    if request.locked().len() == 0 && request.dependencies().len() == 0 {
        exit_with!("nothing to do")
    }

    let mut prompt = false;

    if request.locked().len() != 0 {
        println!("You're installing the following packages:");
        for x in request.locked() {
            println!("- {} version {}", x.name(), x.version());
        }
        prompt = prompt || request.locked().len() > 1;
    }

    if request.legacy_folders().len() != 0 || request.legacy_files().len() != 0 {
        println!("You're removing the following legacy assets:");
        for x in request.legacy_folders().iter().chain(request.legacy_files()) {
            println!("- {}", x.display());
        }
        prompt = true;
    }

    if prompt {
        if yes {
            println!("--yes is set. skipping confirm");
        } else {
            if !confirm_prompt("Do you want to continue install?") {
                exit(1);
            }
        }
    }
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

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Install {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let mut env = load_env(client).await;
        let mut unity = load_unity(self.project).await;

        env.load_package_infos().await.exit_context("loading repositories");

        if let Some(name) = self.name {
            let version_selector = match self.version {
                None if self.prerelease => VersionSelector::LatestIncluidingPrerelease,
                None => VersionSelector::Latest,
                Some(ref version) => VersionSelector::Specific(version),
            };
            let package = get_package(&env, &name, version_selector);

            let request = unity.add_package_request(&env, vec![package], true)
                .await
                .exit_context("collecting packages to be installed");

            print_prompt_install(&request, self.yes);

            unity.do_add_package_request(&env, request).await.exit_context("adding package");

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
        let mut env = load_env(client).await;
        let unity = load_unity(self.project).await;

        env.load_package_infos().await.exit_context("loading repositories");

        let mut outdated_packages = HashMap::new();

        for (name, dep) in unity.locked_packages() {
            match env.find_package_by_name(name, VersionSelector::Latest)
            {
                None => log::error!("package {} not found.", name),
                // if found version is newer: add to outdated
                Some(pkg) if dep.version < *pkg.version() => {
                    outdated_packages.insert(pkg.name(), (pkg, &dep.version));
                }
                Some(_) => (),
            }
        }

        for (_, dependencies) in unity.all_dependencies() {
            for (name, range) in dependencies {
                if let Some((outdated, _)) = outdated_packages.get(name.as_str()) {
                    if !range.matches(&outdated.version()) {
                        outdated_packages.remove(name.as_str());
                    }
                }
            }
        }

        match self.json_format.map(|x| x.get()).unwrap_or(0) {
            0 => {
                for (name, (found, installed)) in &outdated_packages {
                    println!(
                        "{}: installed: {}, found: {}",
                        name, installed, &found.version()
                    );
                }
            }
            1 => {
                #[derive(Serialize)]
                struct OutdatedInfo<'a> {
                    package_name: &'a str,
                    installed_version: &'a Version,
                    newer_version: &'a Version,
                }
                let info = outdated_packages
                    .into_iter()
                    .map(|(package_name, (found, installed))| OutdatedInfo {
                        package_name,
                        installed_version: installed,
                        newer_version: found.version(),
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

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Upgrade {
    pub async fn run(self) {
        let client = crate::create_client(self.offline);
        let mut env = load_env(client).await;
        let mut unity = load_unity(self.project).await;

        env.load_package_infos().await.exit_context("loading repositories");

        let updates = if let Some(name) = self.name {
            let version_selector = match self.version {
                None if self.prerelease => VersionSelector::LatestIncluidingPrerelease,
                None => VersionSelector::Latest,
                Some(ref version) => VersionSelector::Specific(version),
            };
            let package = get_package(&env, &name, version_selector);

            vec![package]
        } else {
            let version_selector = match self.prerelease {
                true => VersionSelector::LatestIncluidingPrerelease,
                false => VersionSelector::Latest,
            };

            unity.locked_packages()
                .keys()
                .map(|name| get_package(&env, &name, version_selector))
                .collect()
        };

        let request = unity.add_package_request(&env, updates, false)
            .await
            .exit_context("collecting packages to be upgraded");

        print_prompt_install(&request, self.yes);

        let updates = request.locked().iter().map(|x| (x.name().clone(), x.version().clone())).collect::<Vec<_>>();

        unity.do_add_package_request(&env, request).await.exit_context("upgrading packages");

        for (name, version) in updates {
            println!("upgraded {} to {}", name, version);
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
        let mut env = load_env(client).await;

        env.load_package_infos().await.exit_context("loading repositories");

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
            });

        if found_packages.is_empty() {
            println!("No matching package found!")
        } else {
            for x in found_packages {
                if let Some(name) = &x.display_name {
                    println!("{} version {}", name, x.version);
                    println!("({})", x.name);
                } else {
                    println!("{} version {}", x.name, x.version);
                }
                if let Some(description) = &x.description {
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
        let mut env = load_env(client).await;

        env.load_package_infos().await.exit_context("loading repositories");

        for (local_path, repo) in env.get_repo_with_path() {
            println!(
                "{}: {} (at {})",
                repo.id().unwrap_or("(unnamed)"),
                repo.name().unwrap_or("(unnamed)"),
                local_path.display(),
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

/// List packages in specified repository
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
        fn print_repo<'a>(packages: &Repository) {
            for versions in packages.get_packages() {
                if let Some((_, pkg)) = versions.versions.iter().max_by_key(|(_, pkg)| &pkg.version) {
                    let package = &pkg.name;
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
            let repo = download_remote_repository(&client, url, None, None)
                .await
                .exit_context("downloading repository")
                .unwrap()
                .0;

            print_repo(&repo);
        } else {
            let mut env = load_env(client).await;

            env.load_package_infos().await.exit_context("loading repositories");

            let some_name = Some(self.name_or_url.as_str());
            let mut found = false;

            for repo in env.get_repos() {
                if repo.name() == some_name {
                    print_repo(repo.repo());
                    found = true;
                }
            }

            if !found {
                exit_with!("no repository named {} found!", self.name_or_url);
            }
        }
    }
}
