mod changes;
mod config;
mod packages;
mod projects;
mod settings;
mod updater;

pub use changes::*;
pub use config::*;
pub use packages::*;
pub use projects::*;
pub use settings::*;
pub use updater::*;

pub fn new_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("vrc-get-litedb/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("building client")
}
