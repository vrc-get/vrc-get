use crate::version::Version;
use crate::vpm::VersionSelector;
use clap::{Parser, Subcommand};
use reqwest::Url;
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
    path_or_url: String,
}

impl Repo {
    pub async fn run(self) {
        let client = crate::create_client();
        let mut env = crate::vpm::Environment::load_default(client)
            .await
            .expect("loading global config");

        if let Ok(url) = Url::parse(&self.path_or_url) {
            env.add_remote_repo(url).await.expect("adding repository")
        } else {
            env.add_local_repo(Path::new(&self.path_or_url))
                .await
                .expect("adding repository")
        }

        env.save().await.expect("saving settings file");
    }
}
