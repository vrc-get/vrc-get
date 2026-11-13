/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

mod changes;
mod config;
mod packages;
mod settings;
mod templates;
mod unity;
mod updater;

pub use changes::*;
pub use config::*;
pub use packages::*;
pub use settings::*;
pub use templates::*;
pub use unity::*;
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
