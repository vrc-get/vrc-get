use clap::{Args, Parser, Subcommand};
use indexmap::IndexMap;
use itertools::Itertools;

use log::warn;
use reqwest::header::{HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue};
use reqwest::{Client, Url};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::error::Error as StdError;
use std::ffi::OsStr;
use std::fmt::{Debug, Display};
use std::num::NonZeroU32;
use std::path::{Component, Path, PathBuf};
use std::process::exit;
use std::str::FromStr;
use vrc_get_vpm::io::{DefaultEnvironmentIo, DefaultProjectIo};
use vrc_get_vpm::repository::RemoteRepository;
use vrc_get_vpm::unity_project::pending_project_changes::{PackageChange, RemoveReason};
use vrc_get_vpm::unity_project::{AddPackageOperation, PendingProjectChanges};
use vrc_get_vpm::version::Version;
use vrc_get_vpm::{PackageCollection, PackageInfo, PackageJson, UserRepoSetting, VersionSelector};

type Environment = vrc_get_vpm::Environment<Client, DefaultEnvironmentIo>;
type UnityProject = vrc_get_vpm::UnityProject<DefaultProjectIo>;

macro_rules! multi_command {
    ($class: ident is $($args:tt)*) => {
        multi_command!(@ fn run $class [$($args)*] []);
    };
    (fn $f: ident $class: ident is $($args:tt)*) => {
        multi_command!(@ fn $f $class [$($args)*] []);
    };

    (@ fn $f: ident $class: ident [$variant: ident $(, $($args:tt)*)?] [$($out:tt)*]) => {
        multi_command!(@ fn $f $class [$($($args)*)?] [
            $($out)*
            $class::$variant(cmd) => cmd.run().await,
        ]);
    };

    (@ fn $f: ident $class: ident [#[$meta:meta] $variant: ident $(, $($args:tt)*)?] [$($out:tt)*]) => {
        multi_command!(@ fn $f $class [$($($args)*)?] [
            $($out)*
            #[$meta]
            $class::$variant(cmd) => cmd.run().await,
        ]);
    };

    (@ fn $f: ident $class: ident [] [$($out:tt)*]) => {
        impl $class {
            pub async fn $f(self) {
                match self {
                    $($out)*
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
    let io = DefaultEnvironmentIo::new_default();
    let mut env = Environment::load(client, io)
        .await
        .exit_context("loading global config");

    env.load_package_infos(!args.no_update)
        .await
        .exit_context("loading repositories");
    env.save().await.exit_context("saving repositories updates");

    env
}

async fn load_unity(path: Option<Box<Path>>) -> UnityProject {
    let io = match path {
        None => {
            let current_dir = env::current_dir().exit_context("getting current directory");
            DefaultProjectIo::find_project_parent(current_dir).exit_context("finding unity project")
        }
        Some(path) => DefaultProjectIo::new(path),
    };

    UnityProject::load(io)
        .await
        .exit_context("loading unity project")
}

fn get_package<'env>(
    env: &'env Environment,
    name: &str,
    selector: VersionSelector,
) -> PackageInfo<'env> {
    env.find_package_by_name(name, selector)
        .unwrap_or_else(|| exit_with!("no matching package not found"))
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

fn print_prompt_install(changes: &PendingProjectChanges) {
    if changes.package_changes().is_empty() {
        exit_with!("nothing to do")
    }

    let mut newly_installed = Vec::new();
    let mut adding_to_dependencies = Vec::new();
    let mut removed = Vec::new();

    for (name, change) in changes.package_changes() {
        match change {
            PackageChange::Install(change) => {
                if let Some(package) = change.install_package() {
                    newly_installed.push(package);
                }
                if let Some(v) = change.to_dependencies() {
                    adding_to_dependencies.push((name, v));
                }
            }
            PackageChange::Remove(change) => {
                removed.push((change.reason(), name));
            }
        }
    }

    if !newly_installed.is_empty() {
        println!("You're installing the following packages:");
        for x in &newly_installed {
            if x.is_yanked() {
                println!("- {} version {} (yanked)", x.name(), x.version());
            } else {
                println!("- {} version {}", x.name(), x.version());
            }
        }
    }

    if !adding_to_dependencies.is_empty() {
        println!("You're adding the following packages to dependencies:");
        for (name, range) in &adding_to_dependencies {
            println!("- {} version {}", name, range);
        }
    }

    if !changes.remove_legacy_folders().is_empty() || !changes.remove_legacy_files().is_empty() {
        println!("You're removing the following legacy assets:");
        for x in changes
            .remove_legacy_folders()
            .iter()
            .chain(changes.remove_legacy_files())
        {
            println!("- {}", x.display());
        }
    }

    if !removed.is_empty() {
        println!("You're removing the following packages:");
        removed.sort_by_key(|(reason, _)| *reason);
        for (reason, name) in removed {
            let reason_name = match reason {
                RemoveReason::Requested => "requested",
                RemoveReason::Legacy => "legacy",
                RemoveReason::Unused => "unused",
            };
            println!("- {} (removed since {})", name, reason_name);
        }
    }

    // process package conflicts
    {
        let mut conflicts = (changes.conflicts().iter())
            .filter(|(_, conflicts)| !conflicts.conflicting_packages().is_empty())
            .peekable();

        if conflicts.peek().is_some() {
            println!("**Those changes conflicts with the following packages**");

            for (package, conflicts) in conflicts {
                println!("{package} conflicts with:");
                for conflict in conflicts.conflicting_packages() {
                    println!("- {conflict}");
                }
            }
        }
    }

    // process unity conflicts
    {
        let mut unity_conflicts = (changes.conflicts().iter())
            .filter(|(_, conflicts)| conflicts.conflicts_with_unity())
            .map(|(package, _)| package)
            .peekable();

        if unity_conflicts.peek().is_some() {
            println!("**Those packages are incompatible with your unity version**");
            for package in unity_conflicts {
                println!("- {}", package);
            }
        }
    }
}

fn prompt_install(yes: bool) {
    if yes {
        println!("--yes is set. skipping confirm");
    } else if !confirm_prompt("Do you want to apply those changes?") {
        exit(1);
    }
}

fn require_prompt_for_install(
    changes: &PendingProjectChanges,
    name: &str,
    version: Option<&Version>,
) -> bool {
    // dangerous changes
    if !changes.remove_legacy_folders().is_empty()
        || !changes.remove_legacy_files().is_empty()
        || !changes.conflicts().is_empty()
    {
        return true;
    }

    // unintended changes
    let Some((change_name, changes)) = changes.package_changes().iter().exactly_one().ok() else {
        return true;
    };

    if change_name.as_ref() != name {
        return true;
    }

    let Some(install) = changes.as_install() else {
        return true;
    };

    // if we're installing package,
    if let Some(package) = install.install_package() {
        if let Some(request_version) = version {
            if request_version != package.version() {
                return true;
            }
        }
    }

    false
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
mod migrate;
#[cfg(feature = "experimental-vcc")]
mod vcc;

/// Open Source command line interface of VRChat Package Manager.
#[derive(Parser)]
#[command(author, version, about)]
pub enum Command {
    #[command(alias = "i")]
    Install(Install),
    Resolve(Resolve),
    #[command(alias = "rm")]
    Remove(Remove),
    Update(Update),
    Outdated(Outdated),
    Upgrade(Upgrade),
    Downgrade(Downgrade),
    Search(Search),
    #[command(subcommand)]
    Repo(Repo),
    #[command(subcommand)]
    Info(info::Info),
    #[command(subcommand)]
    Migrate(migrate::Migrate),
    #[cfg(feature = "experimental-vcc")]
    #[command(subcommand)]
    Vcc(vcc::Vcc),

    Completion(Completion),
}

multi_command!(Command is
    Install,
    Resolve,
    Remove,
    Update,
    Outdated,
    Upgrade,
    Downgrade,
    Search,
    Repo,
    Info,
    Migrate,
    #[cfg(feature = "experimental-vcc")] Vcc,
    Completion,
);

/// Adds package to unity project
///
/// With install command, you'll add to dependencies. With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version)]
pub struct Install {
    /// id of Package
    #[arg()]
    id: Option<String>,
    /// Version of package. if not specified, latest version will be used
    #[arg(id = "VERSION")]
    version: Option<Version>,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// Install package by display name instead of name
    ///
    /// This option is experimental and behavior may change in the future.
    #[arg(long = "name", short = 'n')]
    name: bool,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[command(flatten)]
    env_args: EnvArgs,

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Install {
    pub async fn run(self) {
        let Some(name) = self.id else {
            // if resolve
            return Resolve {
                project: self.project,
                env_args: self.env_args,
            }
            .run()
            .await;
        };

        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;

        let version_selector = match self.version {
            None => VersionSelector::latest_for(unity.unity_version(), self.prerelease),
            Some(ref version) => VersionSelector::specific_version(version),
        };
        let packages = if self.name {
            warn!("--name is experimental and behavior may change in the future.");

            fn normalize_name(name: &str) -> String {
                name.chars()
                    .map(|x| x.to_ascii_lowercase())
                    .filter(|x| !x.is_ascii_whitespace())
                    .collect::<String>()
            }

            let normalized = normalize_name(&name);
            let packages = env.find_whole_all_packages(version_selector, |pkg| {
                pkg.display_name().map(normalize_name).as_ref() == Some(&normalized)
                    || pkg
                        .aliases()
                        .iter()
                        .map(Box::as_ref)
                        .any(|x| normalize_name(x) == normalized)
            });
            if packages.is_empty() {
                exit_with!("no matching package not found")
            }
            packages.into_iter().unique_by(|x| x.name()).collect()
        } else {
            vec![get_package(&env, &name, version_selector)]
        };

        let changes = unity
            .add_package_request(
                &env,
                packages,
                AddPackageOperation::InstallToDependencies,
                self.prerelease,
            )
            .await
            .exit_context("collecting packages to be installed");

        print_prompt_install(&changes);

        if require_prompt_for_install(&changes, name.as_str(), None) {
            prompt_install(self.yes);
        }

        unity
            .apply_pending_changes(&env, changes)
            .await
            .exit_context("adding package");

        unity.save().await.exit_context("saving manifest file");
    }
}

/// (re)installs all locked packages
///
/// If some install packages that is not locked depends on non installed packages,
/// This command tries to install those packages.
#[derive(Parser)]
#[command(author, version)]
pub struct Resolve {
    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[command(flatten)]
    env_args: EnvArgs,
}

impl Resolve {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;

        let changes = unity
            .resolve_request(&env)
            .await
            .exit_context("collecting packages to be installed");

        print_prompt_install(&changes);

        unity
            .apply_pending_changes(&env, changes)
            .await
            .exit_context("installing packages");

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
    project: Option<Box<Path>>,
    #[command(flatten)]
    env_args: EnvArgs,

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Remove {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;

        let changes = unity
            .remove_request(&self.names.iter().map(String::as_ref).collect::<Vec<_>>())
            .await
            .exit_context("collecting packages to be removed");

        print_prompt_install(&changes);

        let confirm =
            changes.package_changes().len() >= self.names.len() || !changes.conflicts().is_empty();

        if confirm {
            prompt_install(self.yes);
        }

        unity
            .apply_pending_changes(&env, changes)
            .await
            .exit_context("removing packages");

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
    project: Option<Box<Path>>,
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

        let selector = VersionSelector::latest_for(unity.unity_version(), self.prerelease);

        for locked in unity.locked_packages() {
            match env.find_package_by_name(locked.name(), selector) {
                None => log::error!("latest version for package {} not found.", locked.name()),
                // if found version is newer: add to outdated
                Some(pkg) if locked.version() < pkg.version() => {
                    outdated_packages.insert(pkg.name(), (pkg, locked.version()));
                }
                Some(_) => (),
            }
        }

        for locked in unity.all_packages() {
            for (name, range) in locked.dependencies() {
                if let Some((outdated, _)) = outdated_packages.get(name.as_ref()) {
                    if !range.matches(outdated.version()) {
                        outdated_packages.remove(name.as_ref());
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
    project: Option<Box<Path>>,
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

        let updates = if let Some(name) = &self.name {
            let version_selector = match self.version {
                None => VersionSelector::latest_for(unity.unity_version(), self.prerelease),
                Some(ref version) => VersionSelector::specific_version(version),
            };
            let package = get_package(&env, name, version_selector);

            vec![package]
        } else {
            let version_selector =
                VersionSelector::latest_for(unity.unity_version(), self.prerelease);

            unity
                .locked_packages()
                .map(|locked| get_package(&env, locked.name(), version_selector))
                .collect()
        };

        let changes = unity
            .add_package_request(
                &env,
                updates,
                AddPackageOperation::UpgradeLocked,
                self.prerelease,
            )
            .await
            .exit_context("collecting packages to be upgraded");

        print_prompt_install(&changes);

        let require_prompt = if let Some(name) = &self.name {
            require_prompt_for_install(&changes, name.as_str(), None)
        } else {
            true
        };

        if require_prompt {
            prompt_install(self.yes)
        }

        let updates = (changes.package_changes().iter())
            .filter_map(|(_, x)| x.as_install())
            .filter_map(|x| x.install_package())
            .map(|x| (x.name().to_owned(), x.version().clone()))
            .collect::<Vec<_>>();

        unity
            .apply_pending_changes(&env, changes)
            .await
            .exit_context("upgrading packages");

        for (name, version) in updates {
            println!("upgraded {} to {}", name, version);
        }

        save_unity(&mut unity).await;
    }
}

/// Downgrade the specified package specified version.
///
/// With install command, you'll add to dependencies. With upgrade command,
/// you'll upgrade dependencies or locked dependencies but not add to dependencies.
#[derive(Parser)]
#[command(author, version)]
pub struct Downgrade {
    /// Name of Package
    #[arg()]
    name: String,
    /// Version of package.
    #[arg(id = "VERSION")]
    version: Version,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<Box<Path>>,
    #[command(flatten)]
    env_args: EnvArgs,

    /// skip confirm
    #[arg(short, long)]
    yes: bool,
}

impl Downgrade {
    pub async fn run(self) {
        let env = load_env(&self.env_args).await;
        let mut unity = load_unity(self.project).await;

        let updates = vec![get_package(
            &env,
            &self.name,
            VersionSelector::specific_version(&self.version),
        )];

        let changes = unity
            .add_package_request(
                &env,
                updates,
                AddPackageOperation::Downgrade,
                self.prerelease,
            )
            .await
            .exit_context("collecting packages to be upgraded");

        print_prompt_install(&changes);

        if require_prompt_for_install(&changes, self.name.as_str(), None) {
            prompt_install(self.yes)
        }

        let downgrades = (changes.package_changes().iter())
            .filter_map(|(_, x)| x.as_install())
            .filter_map(|x| x.install_package())
            .map(|x| (x.name().to_owned(), x.version().clone()))
            .collect::<Vec<_>>();

        unity
            .apply_pending_changes(&env, changes)
            .await
            .exit_context("upgrading packages");

        for (name, version) in downgrades {
            println!("downgraded {} to {}", name, version);
        }

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

            sources.push(pkg.name().to_ascii_lowercase());
            sources.extend(pkg.display_name().map(|x| x.to_ascii_lowercase()));
            sources.extend(pkg.description().map(|x| x.to_ascii_lowercase()));

            sources
        }

        let found_packages =
            env.find_whole_all_packages(VersionSelector::latest_for(None, true), |pkg| {
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
                if let Some(name) = x.package_json().display_name() {
                    println!("{} version {}", name, x.version());
                    println!("({})", x.name());
                } else {
                    println!("{} version {}", x.name(), x.version());
                }
                if let Some(description) = x.package_json().description() {
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

        for (local_path, repo) in env.get_repos() {
            println!(
                "{}: {} (from {} at {})",
                repo.id()
                    .or(repo.url().map(Url::as_str))
                    .unwrap_or("(no id)"),
                repo.name().unwrap_or("(unnamed)"),
                repo.url().map(Url::as_str).unwrap_or("(no remote)"),
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
        let (name, value) = s.split_once(':').ok_or(HeaderPairErr::NoComma)?;
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
            let mut headers = IndexMap::<Box<str>, Box<str>>::new();
            for HeaderPair(name, value) in self.header {
                headers.insert(name.as_str().into(), value.to_str().unwrap().into());
            }
            env.add_remote_repo(url, self.name.as_deref(), headers)
                .await
                .exit_context("adding repository")
        } else {
            let cwd = env::current_dir().exit_context("getting current directory");
            let joined = cwd.join(&self.path_or_url);
            let normalized = normalize_path(&joined);
            if !normalized.exists() {
                exit_with!("path not found: {}", normalized.display());
            }
            env.add_local_repo(normalized.as_ref(), self.name.as_deref())
                .exit_context("adding repository")
        }

        save_env(&mut env).await;
    }
}

fn normalize_path(input: &Path) -> PathBuf {
    let mut result = PathBuf::with_capacity(input.as_os_str().len());

    for component in input.components() {
        match component {
            Component::Prefix(prefix) => result.push(prefix.as_os_str()),
            Component::RootDir => result.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            Component::Normal(_) => result.push(component.as_os_str()),
        }
    }

    result
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
            RepoSearcher::Id => repo.id().map(OsStr::new),
            RepoSearcher::Url => repo.url().map(|x| OsStr::new(x.as_str())),
            RepoSearcher::Name => repo.name().map(OsStr::new),
            RepoSearcher::Path => Some(repo.local_path().as_os_str()),
        }
    }
}

impl RepoRemove {
    pub async fn run(self) {
        let mut env = load_env(&self.env_args).await;

        // we're using OsStr for paths.
        let finder = OsStr::new(self.finder.as_str());
        let searcher = self.searcher.as_searcher();

        let count = env.remove_repo(|x| searcher.get(x) == Some(finder)).await;

        println!("removed {} repositories with {}", count, searcher);

        save_env(&mut env).await;
    }
}

/// Cleanup repositories in Repos directory
///
/// The official VPM CLI will add &lt;uuid&gt;.json in the Repos directory even if error occurs.
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
        env.cleanup_repos_folder()
            .await
            .exit_context("cleaning up Repos directory");
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
        fn print_repo(packages: &RemoteRepository) {
            for versions in packages.get_packages() {
                if let Some(pkg) = versions.get_latest(VersionSelector::latest_for(None, true)) {
                    if let Some(display_name) = pkg.display_name() {
                        println!("{} | {}", display_name, pkg.name());
                    } else {
                        println!("{}", pkg.name());
                    }
                    if let Some(description) = pkg.description() {
                        println!("{}", description);
                    }
                    let mut versions = versions.all_versions().collect::<Vec<_>>();
                    versions.sort_by_key(|pkg| pkg.version());
                    for pkg in &versions {
                        println!(
                            "{}: {}",
                            pkg.version(),
                            pkg.url().map(|x| x.as_str()).unwrap_or("<no url>")
                        );
                    }
                    println!();
                }
            }
        }

        if let Ok(url) = Url::parse(&self.name_or_url) {
            if self.env_args.offline {
                exit_with!("remote repository specified but offline mode.");
            }
            let client = crate::create_client(self.env_args.offline).unwrap();
            let (repo, _) = RemoteRepository::download(&client, &url, &IndexMap::new())
                .await
                .exit_context("downloading repository");

            print_repo(&repo);
        } else {
            let env = load_env(&self.env_args).await;

            let some_name = Some(self.name_or_url.as_str());
            let mut found = false;

            for (_, repo) in env.get_repos() {
                if repo.name() == some_name || repo.id() == some_name {
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

        let Some(shell) = self.shell.or_else(clap_complete::Shell::from_env) else {
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
