/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::common::get_temp_path;
use vrc_get_vpm::environment::{Settings, cleanup_repos_folder};
use vrc_get_vpm::io::DefaultEnvironmentIo;

mod common;

#[tokio::test]
async fn preserves_ignored_builtin_repository_caches() {
    let environment_path = get_temp_path("preserves_ignored_builtin_repository_caches");
    match tokio::fs::remove_dir_all(&environment_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!(
            "failed to clean test directory '{}': {error}",
            environment_path.display()
        ),
    }

    let repos_path = environment_path.join("Repos");
    tokio::fs::create_dir_all(&repos_path).await.unwrap();

    for file_name in [
        "vrc-official.json",
        "vrc-curated.json",
        "package-cache.json",
        "unused.json",
    ] {
        tokio::fs::write(repos_path.join(file_name), b"{}")
            .await
            .unwrap();
    }

    let io = DefaultEnvironmentIo::new(environment_path.into_boxed_path());
    let mut settings = Settings::load(&io).await.unwrap();
    settings.set_ignore_official_repository(true);
    settings.set_ignore_curated_repository(true);

    cleanup_repos_folder(&settings, &io).await.unwrap();

    assert!(repos_path.join("vrc-official.json").is_file());
    assert!(repos_path.join("vrc-curated.json").is_file());
    assert!(repos_path.join("package-cache.json").is_file());
    assert!(!repos_path.join("unused.json").exists());
}
