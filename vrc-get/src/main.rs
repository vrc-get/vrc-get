#![deny(clippy::wildcard_enum_match_arm)]

extern crate core;

use clap::Parser;
use reqwest::Client;

mod commands;

#[tokio::main]
async fn main() {
    init_log();
    commands::Command::parse().run().await;
}

fn init_log() {
    use env_logger::fmt::style;
    use env_logger::*;
    use log::Level;
    use std::io::Write as _;

    let env = Env::default().filter_or(DEFAULT_FILTER_ENV, "info");
    let use_custom_format = std::env::var_os(DEFAULT_FILTER_ENV).is_none()
        && std::env::var_os(DEFAULT_WRITE_STYLE_ENV).is_none();
    let mut builder = Builder::from_env(env);
    if use_custom_format {
        // if none of the env vars are set, use the custom format
        builder.format(|buf, record| {
            let (prefix, style) = match record.level() {
                Level::Error => ("e:", style::AnsiColor::Red.on_default()),
                Level::Warn => ("w:", style::AnsiColor::Yellow.on_default()),
                Level::Info => ("i:", style::AnsiColor::White.on_default().bold()),
                // should not reach but just in case
                Level::Debug => return Ok(()),
                Level::Trace => return Ok(()),
            };

            let render = style.render();
            let render_reset = style.render_reset();
            writeln!(buf, "{render}{prefix}{render_reset} {}", record.args())
        });
    }
    builder.init();
}

pub(crate) fn create_client(offline: bool) -> Option<Client> {
    if offline {
        None
    } else {
        let authors = env!("CARGO_PKG_AUTHORS");
        let author = authors.split_once(':').unwrap_or(("", authors)).1;
        let user_agent = format!(
            "{product}/{version} (a open-source command-line vpm client by {author})",
            product = env!("CARGO_PKG_NAME"),
            version = env!("CARGO_PKG_VERSION"),
            author = author
        );

        log::debug!("using user agent: {user_agent}");

        Some(
            Client::builder()
                .user_agent(user_agent)
                .build()
                .expect("building client"),
        )
    }
}
