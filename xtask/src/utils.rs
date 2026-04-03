#![allow(dead_code)]

use std::sync::OnceLock;

pub mod cargo;
pub mod command;
pub mod rustc;

pub fn ureq() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

    AGENT.get_or_init(|| {
        ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .user_agent("cargo-xtask of vrc-get (https://github.com/vrc-get/vrc-get)")
                .build(),
        )
    })
}
