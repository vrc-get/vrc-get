/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use anyhow::*;

mod alcom_updater_json;
mod build_alcom;
mod bundle_alcom;
mod check_static_link;
mod sign_alcom_app;
mod sign_alcom_updater;
mod utils;

trait Command {
    fn run(self) -> Result<i32>;
}

macro_rules! commands_def {
    (
        $(
        $(#[$attr:meta])*
        $name: ident = $module: ident;
        )*
    ) => {
        #[derive(clap::Parser)]
        enum Commands {
            $($(#[$attr])* $name($module::Command),)*
        }

        impl Command for Commands {
            fn run(self) -> Result<i32> {
                match self {
                    $(Commands::$name(cmd) => Command::run(cmd),)*
                }
            }
        }
    };
}

commands_def! {
    CheckStaticLink = check_static_link;
    AlcomUpdaterJson = alcom_updater_json;
    BuildAlcom = build_alcom;
    BundleAlcom = bundle_alcom;
    SignAlcomApp = sign_alcom_app;
    SignAlcomUpdater = sign_alcom_updater;
}

fn main() -> Result<()> {
    let command: Commands = clap::Parser::parse();
    std::process::exit(command.run()?);
}
