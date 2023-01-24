use crate::version::Version;
use crate::vpm::structs::remote_repo::PackageVersions;
use crate::vpm::{download_remote_repository, VersionSelector};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde_json::{from_value, Map, Value};
use std::ffi::{OsStr, OsString};
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

/// Open Source command line interface of VRChat Package Manager.
#[derive(Parser)]
pub enum Command {
    Install(Install),
    #[command(subcommand)]
    Repo(Repo),
}

multi_command!(Command is Install, Repo);

#[derive(Parser)]
pub struct Install {
    /// Name of Package
    #[arg()]
    name: Option<String>,
    /// Version of package. if not specified, latest version will be used
    #[arg()]
    version: Option<Version>,
    /// Include prerelease
    #[arg(long = "prerelease")]
    prerelease: bool,

    /// Path to project dir. by default CWD or parents of CWD will be used
    #[arg(short = 'p', long = "project")]
    project: Option<PathBuf>,
}

impl Install {
    pub async fn run(self) {
        let client = crate::create_client();
        let env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");
        let mut unity = crate::vpm::UnityProject::find_unity_project(self.project)
            .await
            .expect("unity project not found");

        if let Some(name) = self.name {
            let version_selector = match self.version {
                None if self.prerelease => VersionSelector::LatestIncluidingPrerelease,
                None => VersionSelector::Latest,
                Some(ref version) => VersionSelector::Specific(version),
            };
            let package = env
                .find_package_by_name(&name, version_selector)
                .await
                .expect("finding package")
                .expect("no matching package not found");
            unity
                .add_package(&env, &package)
                .await
                .expect("adding package");
        } else {
            unity.resolve(&env).await.expect("resolving");
        }

        unity.save().await.expect("saving manifest file");
    }
}

/// Commands around repositories
#[derive(Subcommand)]
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
pub struct RepoList {}

impl RepoList {
    pub async fn run(self) {
        let client = crate::create_client();
        let env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        for repo in env.get_repos().await.expect("getting repo list") {
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
pub struct RepoAdd {
    /// URL of Package
    #[arg()]
    path_or_url: String,
    /// Name of Package
    #[arg()]
    name: Option<String>,
}

impl RepoAdd {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        if let Ok(url) = Url::parse(&self.path_or_url) {
            env.add_remote_repo(url, self.name.as_deref())
                .await
                .expect("adding repository")
        } else {
            env.add_local_repo(Path::new(&self.path_or_url), self.name.as_deref())
                .await
                .expect("adding repository")
        }

        env.save().await.expect("saving settings file");
    }
}

/// Remove repository with specified url, path or name
#[derive(Parser)]
pub struct RepoRemove {
    /// URL of Package
    #[arg()]
    name_or_url: String,
}

impl RepoRemove {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        let removed = if let Ok(url) = Url::parse(&self.name_or_url) {
            env.remove_repo(|x| x.url.as_deref() == Some(url.as_str()))
                .await
                .expect("removing based on url")
        } else {
            let path = Path::new(&self.name_or_url);
            env.remove_repo(|x| x.local_path.as_path() == path)
                .await
                .expect("removing based on path")
        };

        if !removed {
            env.remove_repo(|x| x.name.as_deref() == Some(self.name_or_url.as_str()))
                .await
                .expect("removing based on name");
        }

        env.save().await.expect("saving settings file");
    }
}

/// Cleanup repositories in Repos directory
///
/// The official VPM CLI will add <uuid>.json in the Repos directory even if error occurs.
/// So this command will cleanup Repos directory.
#[derive(Parser)]
pub struct RepoCleanup {}

impl RepoCleanup {
    pub async fn run(self) {
        let client = crate::create_client();
        let env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        let mut uesr_repo_file_names = vec![
            OsString::from("vrc-official.json"),
            OsString::from("vrc-curated.json"),
        ];
        let repos_base = env.get_repos_dir();

        for x in env.get_user_repos().expect("userRepos") {
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

        let mut entry = read_dir(repos_base).await.expect("reading dir");
        while let Some(entry) = entry.next_entry().await.expect("reading dir") {
            let path = entry.path();
            if tokio::fs::metadata(&path)
                .await
                .expect("metadata")
                .is_file()
                && path.extension() == Some(OsStr::new("json"))
                && !uesr_repo_file_names.contains(&entry.file_name())
            {
                remove_file(path).await.expect("reading dir");
            }
        }
    }
}

/// Remove repository from user repositories.
#[derive(Parser)]
pub struct RepoPackages {
    name_or_url: String,
}

impl RepoPackages {
    pub async fn run(self) {
        fn print_repo(cache: Map<String, Value>) {
            for (package, value) in cache {
                let versions = from_value::<PackageVersions>(value).expect("loading package data");
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

        let client = crate::create_client();

        if let Some(url) = Url::parse(&self.name_or_url).ok() {
            let repo = download_remote_repository(&client, url)
                .await
                .expect("downloading repository");

            let cache = repo
                .get("packages")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or(Map::<String, Value>::new());

            print_repo(cache);
        } else {
            let env = crate::vpm::Environment::load_default(client)
                .await
                .expect("loading global config");
            let some_name = Some(self.name_or_url.as_str());
            let mut found = false;

            for repo in env.get_repos().await.expect("listing packages") {
                if repo.creation_info.as_ref().and_then(|x| x.name.as_deref()) == some_name
                    || repo.description.as_ref().and_then(|x| x.name.as_deref()) == some_name
                {
                    print_repo(repo.cache.clone());
                    found = true;
                }
            }

            if !found {
                eprintln!("no repository named {} found!", self.name_or_url);
                exit(1);
            }
        }
    }
}
