use crate::version::Version;
use crate::vpm::structs::remote_repo::PackageVersions;
use crate::vpm::{download_remote_repository, VersionSelector};
use clap::{Parser, Subcommand};
use reqwest::Url;
use serde_json::{from_value, Map, Value};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum Command {
    Repos(Repos),
    Repo(Repo),
}

impl Command {
    pub async fn run(self) {
        match self {
            Command::Repos(cmd) => cmd.run().await,
            Command::Repo(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser)]
pub struct Repos {}

impl Repos {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        env.get_repos(|repo| {
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
            Ok(())
        })
        .await
        .expect("error listing repo");
    }
}

#[derive(Parser)]
pub struct Repo {
    /// Name of Package
    #[arg()]
    url: String,
}

impl Repo {
    pub async fn run(self) {
        let client = crate::create_client();

        let repo = download_remote_repository(&client, self.url)
            .await
            .expect("downloading repository");

        let cache = repo
            .get("packages")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or(Map::<String, Value>::new());

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
}
