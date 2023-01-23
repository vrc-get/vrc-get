use clap::Parser;
use reqwest::Client;

mod commands;
mod version;
mod vpm;

#[tokio::main]
async fn main() {
    env_logger::init();
    commands::Command::parse().run().await;
}

pub(crate) fn create_client() -> Client {
    Client::builder()
        .user_agent(concat!("vrc-get/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client")
}
