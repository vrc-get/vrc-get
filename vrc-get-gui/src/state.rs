mod changes;
mod config;
mod packages;
mod settings;
mod templates;
mod updater;

pub use changes::*;
pub use config::*;
pub use packages::*;
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
        // https://github.com/vrc-get/vrc-get/issues/2653
        // IDK why but it might take over 10 sec to connect / read
        .connect_timeout(std::time::Duration::from_secs(60))
        .read_timeout(std::time::Duration::from_secs(60))
        .timeout(std::time::Duration::from_secs(10 * 60)) // 10 minutes
        .build()
        .expect("building client")
}
