use clap::{Args, Parser, Subcommand};
use indexmap::IndexMap;
use reqwest::header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue};
use reqwest::Url;
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::ffi::{OsStr, OsString};
use std::fmt::{Debug, Display};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use tokio::fs::{read_dir, remove_file};
use vrc_get_vpm::structs::package::PackageJson;
use vrc_get_vpm::structs::repository::Repository;
use vrc_get_vpm::structs::setting::UserRepoSetting;
use vrc_get_vpm::version::Version;
#[cfg(feature = "experimental-override-predefined")]
use vrc_get_vpm::PreDefinedRepoSource;
use vrc_get_vpm::{
    download_remote_repository, AddPackageRequest, Environment, PackageInfo, PackageSelector,
    UnityProject,
};

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
        ::std::process::exit(1)
    }};
}

#[derive(Args, Default)]
struct EnvArgs {
    /// do not connect to remote servers, use local caches only. implicitly --no-update
    #[arg(long)]
    offline: bool,
    /// do not update local repository cache.
    #[arg(long)]
    no_update: bool,
}

async fn load_env(args: &EnvArgs) -> Environment {
    let client = crate::create_client(args.offline);
    let mut env = Environment::load_default(client)
        .await
        .exit_context("loading global config");

    env.load_package_infos(!args.no_update)
        .await
        .exit_context("loading repositories");
    env.save().await.exit_context("saving repositories updates");

    #[cfg(feature = "experimental-override-predefined")]
    if let Some(url_override) = std::env::var("VRC_GET_OFFICIAL_URL_OVERRIDE").ok() {
        log::warn!("VRC_GET_OFFICIAL_URL_OVERRIDE env variable is set! overriding official repository url is experimental feature!");
        env.set_url_override(PreDefinedRepoSource::Official, url_override);
    }

    #[cfg(feature = "experimental-override-predefined")]
    if let Some(url_override) = std::env::var("VRC_GET_CURATED_URL_OVERRIDE").ok() {
        log::warn!("VRC_GET_CURATED_URL_OVERRIDE env variable is set! overriding official repository url is experimental feature!");
        env.set_url_override(PreDefinedRepoSource::Curated, url_override);
    }

    env
}

async fn load_unity(path: Option<PathBuf>) -> UnityProject {
    UnityProject::find_unity_project(path)
        .await
        .exit_context("loading unity project")
}

fn get_package<'env>(
    env: &'env Environment,
    name: &str,
    selector: PackageSelector,
) -> PackageInfo<'env> {
    env.find_package_by_name(&name, selector)
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
    use std::io;
    use std::io::Write;
    fn _impl(msg: &str) -> io::Result<bool> {
        let mut stdout = io::stdout();
        let stdin = io::stdin();
        let mut buf = String::new();
        loop {
            // prompt
            write!(stdout, "{} [y/n] ", msg)?;
            stdout.flush()?;

            buf.clear();
            stdin.read_line(&mut buf)?;

            buf.make_ascii_lowercase();

            match buf.trim() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => continue,
            }
        }
    }

    _impl(msg).unwrap_or(false)
}

fn print_prompt_install(request: &AddPackageRequest, yes: bool, require_prompt: bool) {
    if request.locked().len() == 0 && request.dependencies().len() == 0 {
        exit_with!("nothing to do")
    }

    let mut prompt = require_prompt;

    if request.locked().len() != 0 {
        println!("You're installing the following packages:");
        for x in request.locked() {
            #[cfg(feature = "experimental-yank")]
            if x.is_yanked() {
                prompt = true;
                println!("- {} version {} (yanked)", x.name(), x.version());
            } else {
                println!("- {} version {}", x.name(), x.version());
            }
            #[cfg(not(feature = "experimental-yank"))]
            println!("- {} version {}", x.name(), x.version());
        }
        prompt = prompt || request.locked().len() > 1;
    }

    if request.legacy_folders().len() != 0 || request.legacy_files().len() != 0 {
        println!("You're removing the following legacy assets:");
        for x in request
            .legacy_folders()
            .iter()
            .chain(request.legacy_files())
        {
            println!("- {}", x.display());
        }
        prompt = true;
    }

    if request.legacy_packages().len() != 0 {
        println!("You're removing the following legacy packages:");
        for x in request.legacy_packages() {
            println!("- {}", x);
        }
        prompt = true;
    }

    if request.conflicts().len() != 0 {
        println!("**Those changes conflicts with the following packages**");
        for (package, conflicts) in request.conflicts() {
            println!("{package} conflicts with:");
            for conflict in conflicts {
                println!("- {conflict}");
            }
        }
        prompt = true;
    }

    if request.unity_conflicts().len() != 0 {
        println!("**Those packages are incompatible with your unity version**");
        for package in request.unity_conflicts() {
            println!("- {package}");
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

mod info;

/// Open Source command line interface of VRChat Package Manager.
#[derive(Parser)]
#[command(author, version, about)]
pub enum Command {
    #[command(alias = "i", alias = "resolve")]
    Install(Install),
    #[command(alias = "rm")]
    Remove(Remove),
    Update(Update),
    Outdated(Outdated),
    Upgrade(Upgrade),
    Search(Search),
    #[command(subcommand)]
    Repo(Repo),
    #[command(subcommand)]
    Info(info::Info),

    Completion(Completion),
}

multi_command!(Command is Install, Remove, Update, Outdated, Upgrade, Search, Repo, Info, Completion);

const INSTALL_USAGE: &str = color_print::cstr!("
      <bold>vrc-get install</>
          installs all packages locked in vpm-manifest.json and dependencies of non-vpm managed embed packages

      <bold>vrc-get install</> [OPTIONS] <<PACKAGE>>
          installs the latest version of specified package

      <bold>vrc-get upgrade</> [OPTIONS] <<PACKAGE>> <<VERSION>>
          installs the specified version of specified package");

/// Adds package to unity project
///
/// With install command, you'll add to dependencies. With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version, override_usage = INSTALL_USAGE)]
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

    /// Path to project dir. by default working directory or parents of working directory will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
    #[command(flatten)]
    env_args: EnvArgs,

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Install {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;

        if let Some(name) = self.name {
            let version_selector = match self.version {
                None => PackageSelector::latest_for(unity.unity_version(), self.prerelease),
                Some(ref version) => PackageSelector::specific_version(version),
            };
            let package = get_package(&env, &name, version_selector);

            let request = unity
                .add_package_request(&env, vec![package], true, self.prerelease)
                .await
                .exit_context("collecting packages to be installed");

            print_prompt_install(&request, self.yes, false);

            unity
                .do_add_package_request(&env, request)
                .await
                .exit_context("adding package");

            mark_and_sweep(&mut unity).await;
        } else {
            let resolve_result = unity.resolve(&env).await.exit_context("resolving packages");
            #[cfg(feature = "experimental-yank")]
            for installed in resolve_result.installed_from_locked() {
                if installed.is_yanked() {
                    eprintln!(
                        "WARN: {} version {} is yanked",
                        installed.name(),
                        installed.version()
                    );
                }
            }
            for installed in resolve_result.installed_from_unlocked_dependencies() {
                println!(
                    "installed {} version {} from dependencies of unlocked packages",
                    installed.name(),
                    installed.version()
                );
            }
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

/// Update local repository cache
#[derive(Parser)]
#[command(author, version)]
pub struct Update {}

impl Update {
    pub async fn run(self) {
        let _ = load_env(&EnvArgs::default()).await;
    }
}

/// Show list of outdated packages
#[derive(Parser)]
#[command(author, version)]
pub struct Outdated {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// With this option, output is printed in json format
    #[arg(long = "json-format")]
    json_format: Option<NonZeroU32>,

    #[command(flatten)]
    env_args: EnvArgs,
}

impl Outdated {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let unity = load_unity(self.project).await;

        let mut outdated_packages = HashMap::new();

        let selector = PackageSelector::latest_for(unity.unity_version(), self.prerelease);

        for (name, dep) in unity.locked_packages() {
            match env.find_package_by_name(name, selector) {
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
                        name,
                        installed,
                        &found.version()
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

const UPGRADE_USAGE: &str = color_print::cstr!("
      <bold>vrc-get upgrade</>
          upgrades all packages to the latest

      <bold>vrc-get upgrade</> [OPTIONS] <<PACKAGE>>
          upgrade specified package to the latest

      <bold>vrc-get upgrade</> [OPTIONS] <<PACKAGE>> <<VERSION>>
          upgrade specified package to the specified version");

/// Upgrade specified package or all packages to latest or specified version.
///
/// With install command, you'll add to dependencies.
/// With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version, override_usage = UPGRADE_USAGE)]
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
    #[command(flatten)]
    env_args: EnvArgs,

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Upgrade {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;
        let require_prompt;

        let updates = if let Some(name) = self.name {
            let version_selector = match self.version {
                None => PackageSelector::latest_for(unity.unity_version(), self.prerelease),
                Some(ref version) => PackageSelector::specific_version(version),
            };
            let package = get_package(&env, &name, version_selector);

            require_prompt = false;

            vec![package]
        } else {
            let version_selector =
                PackageSelector::latest_for(unity.unity_version(), self.prerelease);

            require_prompt = true;

            unity
                .locked_packages()
                .keys()
                .map(|name| get_package(&env, &name, version_selector))
                .collect()
        };

        let request = unity
            .add_package_request(&env, updates, false, self.prerelease)
            .await
            .exit_context("collecting packages to be upgraded");

        print_prompt_install(&request, self.yes, require_prompt);

        let updates = request
            .locked()
            .iter()
            .map(|x| (x.name().clone(), x.version().clone()))
            .collect::<Vec<_>>();

        unity
            .do_add_package_request(&env, request)
            .await
            .exit_context("upgrading packages");

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

    #[command(flatten)]
    env_args: EnvArgs,
}

impl Search {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;

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

        let found_packages = env.find_whole_all_packages(|pkg| {
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
    #[command(flatten)]
    env_args: EnvArgs,
}

impl RepoList {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;

        for (local_path, repo) in env.get_repo_with_path() {
            println!(
                "{}: {} (from {} at {})",
                repo.id().or(repo.url()).unwrap_or("(no id)"),
                repo.name().unwrap_or("(unnamed)"),
                repo.url().unwrap_or("(no remote)"),
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

    /// Headers
    #[arg(short='H', long, value_parser = HeaderPair::from_str)]
    header: Vec<HeaderPair>,

    #[command(flatten)]
    env_args: EnvArgs,
}

#[derive(Clone)]
struct HeaderPair(HeaderName, HeaderValue);

impl FromStr for HeaderPair {
    type Err = HeaderPairErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s.split_once(":").ok_or(HeaderPairErr::NoComma)?;
        Ok(HeaderPair(name.parse()?, value.parse()?))
    }
}

#[derive(Debug)]
enum HeaderPairErr {
    NoComma,
    HeaderNameErr(InvalidHeaderName),
    HeaderValueErr(InvalidHeaderValue),
}

impl From<InvalidHeaderName> for HeaderPairErr {
    fn from(value: InvalidHeaderName) -> Self {
        Self::HeaderNameErr(value)
    }
}

impl From<InvalidHeaderValue> for HeaderPairErr {
    fn from(value: InvalidHeaderValue) -> Self {
        Self::HeaderValueErr(value)
    }
}

impl Display for HeaderPairErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderPairErr::NoComma => f.write_str("no ':' found"),
            HeaderPairErr::HeaderNameErr(e) => Display::fmt(e, f),
            HeaderPairErr::HeaderValueErr(e) => Display::fmt(e, f),
        }
    }
}

impl StdError for HeaderPairErr {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            HeaderPairErr::NoComma => None,
            HeaderPairErr::HeaderNameErr(e) => Some(e),
            HeaderPairErr::HeaderValueErr(e) => Some(e),
        }
    }
}

impl RepoAdd {
    pub async fn run(self) {
        let mut env = load_env(&self.env_args).await;

        if let Ok(url) = Url::parse(&self.path_or_url) {
            let mut headers = IndexMap::<String, String>::new();
            for HeaderPair(name, value) in self.header {
                headers.insert(name.to_string(), value.to_str().unwrap().to_string());
            }
            env.add_remote_repo(url, self.name.as_deref(), headers)
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
    /// id, url, name, or path of repository
    #[arg()]
    finder: String,

    #[clap(flatten)]
    searcher: RepoSearcherArgs,

    #[command(flatten)]
    env_args: EnvArgs,
}

#[derive(Args)]
#[group(multiple = false)]
struct RepoSearcherArgs {
    /// Find repository to remove by id
    #[arg(long)]
    id: bool,
    /// Find repository to remove by url
    #[arg(long)]
    url: bool,
    /// Find repository to remove by name
    #[arg(long)]
    name: bool,
    /// Find repository to remove by local path
    #[arg(long)]
    path: bool,
}

impl RepoSearcherArgs {
    fn as_searcher(&self) -> RepoSearcher {
        match () {
            () if self.id => RepoSearcher::Id,
            () if self.url => RepoSearcher::Url,
            () if self.name => RepoSearcher::Name,
            () if self.path => RepoSearcher::Path,
            () => RepoSearcher::Id,
        }
    }
}

#[derive(Copy, Clone)]
enum RepoSearcher {
    Id,
    Url,
    Name,
    Path,
}

impl Display for RepoSearcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoSearcher::Id => f.write_str("id"),
            RepoSearcher::Url => f.write_str("url"),
            RepoSearcher::Name => f.write_str("name"),
            RepoSearcher::Path => f.write_str("path"),
        }
    }
}

impl RepoSearcher {
    fn get(self, repo: &UserRepoSetting) -> Option<&OsStr> {
        match self {
            RepoSearcher::Id => repo.id.as_deref().map(|x| OsStr::new(x)),
            RepoSearcher::Url => repo.url.as_deref().map(|x| OsStr::new(x)),
            RepoSearcher::Name => repo.name.as_deref().map(|x| OsStr::new(x)),
            RepoSearcher::Path => Some(repo.local_path.as_os_str()),
        }
    }
}

impl RepoRemove {
    pub async fn run(self) {
        let mut env = load_env(&self.env_args).await;

        // we're using OsStr for paths.
        let finder = OsStr::new(self.finder.as_str());
        let searcher = self.searcher.as_searcher();

        let count = env
            .remove_repo(|x| searcher.get(x) == Some(finder))
            .await
            .exit_context("removing repository");

        println!("removed {} repositories with {}", count, searcher);

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
    #[command(flatten)]
    env_args: EnvArgs,
}

impl RepoCleanup {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;

        let mut uesr_repo_file_names = vec![
            OsString::from("vrc-official.json"),
            OsString::from("vrc-curated.json"),
            OsString::from("package-cache.json"),
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

    #[command(flatten)]
    env_args: EnvArgs,
}

impl RepoPackages {
    pub async fn run(self) {
        fn print_repo<'a>(packages: &Repository) {
            for versions in packages.get_packages() {
                if let Some((_, pkg)) = versions.versions.iter().max_by_key(|(_, pkg)| &pkg.version)
                {
                    let package = &pkg.name;
                    if let Some(display_name) = &pkg.display_name {
                        println!("{} | {}", display_name, package);
                    } else {
                        println!("{}", package);
                    }
                    if let Some(description) = &pkg.description {
                        println!("{}", description);
                    }
                    let mut versions = versions.versions.values().collect::<Vec<_>>();
                    versions.sort_by_key(|pkg| &pkg.version);
                    for pkg in &versions {
                        println!("{}: {}", pkg.version, pkg.url);
                    }
                    println!();
                }
            }
        }

        if let Some(url) = Url::parse(&self.name_or_url).ok() {
            if self.env_args.offline {
                exit_with!("remote repository specified but offline mode.");
            }
            let client = crate::create_client(self.env_args.offline).unwrap();
            let repo = download_remote_repository(&client, url, None, None)
                .await
                .exit_context("downloading repository")
                .unwrap()
                .0;

            print_repo(&repo);
        } else {
            let env = load_env(&self.env_args).await;

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

#[derive(Parser)]
pub struct Completion {
    shell: Option<clap_complete::Shell>,
}

impl Completion {
    pub async fn run(self) {
        use clap::CommandFactory;
        use std::env::args;

        let Some(shell) = self.shell.or_else(|| clap_complete::Shell::from_env()) else {
            exit_with!("shell not specified")
        };
        let mut bin_name = args().next().expect("bin name");
        if let Some(slash) = bin_name.rfind(&['/', '\\']) {
            bin_name = bin_name[slash + 1..].to_owned();
        }

        clap_complete::generate(
            shell,
            &mut Command::command(),
            bin_name,
            &mut std::io::stdout(),
        );
    }
}
