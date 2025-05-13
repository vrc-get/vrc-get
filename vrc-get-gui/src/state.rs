mod changes;
mod config;
mod packages;
mod projects;
mod settings;
mod templates;
mod updater;

pub use changes::*;
pub use config::*;
pub use packages::*;
pub use projects::*;
pub use settings::*;
pub use templates::*;
pub use updater::*;

pub fn new_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!(
            "vrc-get-gui/",
            env!("CARGO_PKG_VERSION"),
            " (A GUI version of vrc-get, A.K.K, ALCOM) (",
            env!("CARGO_PKG_HOMEPAGE"),
            ")"
        ))
        .connect_timeout(std::time::Duration::from_secs(10))
        .read_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(10 * 60)) // 10 minutes
        .build()
        .expect("building client")
}
