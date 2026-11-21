/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

//! This module tests the settings.json => vcc.litedb migration behavior.
//! We test migrating fron settings.json => vcc.litedb,
//! The current case and both 1st and 2nd behavior described below will be tested
//!
//! The current VPM toolchain has two places of storing user projects: `settings.json` and `vcc.litedb`.
//! Currently, `settings.json` is the single source of truth, and VCC will always copy
//! information of `settings.json` to `vcc.litedb`.
//!
//! However, it's announced that future VCC will remove the migration process.
//! There's no detailed documentation on how `settings.json` would be when migration removal becomes true.
//! However, we can assume the `userProjects` key will be absent from `settings.json` and `vcc.litedb` become
//! the single source of truth (opposite to current `settings.json`).
//!
//! To support reading the settings.json for both versions and writing for both versions
//! 1) vrc-get will skip copying the data from 'userProjects' to vcc.litedb if 'userProjects' is absent,
//!    for future VCC compatibility
//! 2) vrc-get will always emit 'userProjects' key even if 'userProjects' is absent.
//!    The future VCC will just remove 'userProjects' so this should not cause a problem,
//!    and older VCC will become compatible since 'userProjects' can become single source of truth
//!
//! See https://github.com/vrchat-community/creator-companion/issues/400#issuecomment-1855484391
//! See https://vcc.docs.vrchat.com/news/release-2.2.0/#important-notes-for-tool-developers
//!
//! We use `for iteration in 0..2 {}` to ensure 2nd iteration does not change the data much

#![cfg(feature = "experimental-project-management")]

use crate::common::get_temp_path;
use hex::ToHex;
use itertools::Itertools;
use rusqlite::fallible_iterator::FallibleIterator;
use std::path::{Path, PathBuf};
use vrc_get_litedb::bson::DateTime;
use vrc_get_litedb::file_io::{BsonAutoId, LiteDBFile};
use vrc_get_vpm::ProjectType;
use vrc_get_vpm::environment::{ProjectListMismatch, ProjectManagement};
use vrc_get_vpm::io::DefaultEnvironmentIo;
use vrc_get_vpm::version::UnityVersion;

mod common;

fn clean_dir(path: &Path) {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Ok(()) => {}
        Err(e) => panic!("error cleaning dir {}: {}", path.display(), e),
    }
    std::fs::create_dir_all(path).unwrap();
}

fn normalize_path(path: String) -> String {
    if std::path::MAIN_SEPARATOR != '/' {
        path.replace("/", std::path::MAIN_SEPARATOR_STR)
    } else {
        path
    }
}

fn object_id(bytes: &[u8; 12]) -> vrc_get_litedb::bson::ObjectId {
    vrc_get_litedb::bson::ObjectId::from_bytes(*bytes)
}

const VCC_LITEDB: &str = "vcc.liteDb";
const VRC_GET_SQLITE: &str = "vrc-get/vrc-get.db";
const SETTINGS_JSON: &str = "settings.json";
const VRC_GET_SETTINGS: &str = "vrc-get/settings.json";

macro_rules! test_settings_json_with_projects {
    ($env_projects_str: expr$(,)?) => {
        test_settings_json_with_projects($env_projects_str, vec![])
    };
    ($env_projects_str: expr, additional_projects = $additional_projects:expr$(,)?) => {
        test_settings_json_with_projects($env_projects_str, $additional_projects)
    };
}

fn test_settings_json_with_projects(
    env_projects_str: &str,
    additional_projects: Vec<String>,
) -> String {
    use serde_json::*;
    to_string_pretty(&json!({
        "userProjects": additional_projects.into_iter().chain([
            format!("{env_projects_str}/Blank 2019 project"),
            format!("{env_projects_str}/Blank 2022 project"),
            format!("{env_projects_str}/HistoryOfAvatarOptimizer"),
            format!("{env_projects_str}/VPMPackageAutoInstaller"),
            format!("{env_projects_str}/CrashOnExitWithLogTypeFullName"),
        ]).map(normalize_path).collect::<Vec<_>>()
    }))
    .unwrap()
}

macro_rules! defined_projects_in_settings_json {
    ($env_projects_str: expr$(,)?) => {
        defined_projects_in_settings_json($env_projects_str, vec![])
    };
    ($env_projects_str: expr, additional_projects = [$($path_format: expr),*$(,)?]$(,)?) => {
        defined_projects_in_settings_json($env_projects_str, vec![$($path_format),*])
    };
}

fn defined_projects_in_settings_json(
    env_projects_str: &str,
    additional: Vec<String>,
) -> Vec<String> {
    [
        format!("{env_projects_str}/Blank 2019 project"),
        format!("{env_projects_str}/Blank 2022 project"),
        format!("{env_projects_str}/HistoryOfAvatarOptimizer"),
        format!("{env_projects_str}/VPMPackageAutoInstaller"),
        format!("{env_projects_str}/CrashOnExitWithLogTypeFullName"),
    ]
    .into_iter()
    .chain(additional)
    .map(normalize_path)
    .sorted()
    .collect_vec()
}

fn load_projects_in_settings_json(settings_json: &[u8]) -> Vec<String> {
    serde_json::from_slice::<serde_json::Value>(settings_json).unwrap()["userProjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_string())
        .map(normalize_path)
        .sorted()
        .collect_vec()
}

macro_rules! test_litedb_file_with_projects {
    ($env_projects_str: expr$(,)?) => {
        test_litedb_file_with_projects($env_projects_str, vec![])
    };
    ($env_projects_str: expr, additional_projects = $additional_projects:expr$(,)?) => {
        test_litedb_file_with_projects($env_projects_str, $additional_projects)
    };
}

fn test_litedb_file_with_projects(
    env_projects_str: &str,
    additional_projects: Vec<vrc_get_litedb::bson::Document>,
) -> Vec<u8> {
    let mut litedb = LiteDBFile::new();
    litedb
        .insert(
            "projects",
            vec![
                // "Blank 2019 project" and "Blank 2022 project" are not exist in litedb
                // those three are exist on both litedb and settings.json
                vrc_get_litedb::document! {
                    "Path" => normalize_path(format!("{env_projects_str}/HistoryOfAvatarOptimizer")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => normalize_path(format!("{env_projects_str}/VPMPackageAutoInstaller")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => normalize_path(format!("{env_projects_str}/CrashOnExitWithLogTypeFullName")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                // Those two projects are exist only on litedb
                vrc_get_litedb::document! {
                    "Path" => normalize_path(format!("{env_projects_str}/New Project 32")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => normalize_path(format!("{env_projects_str}/New Project 31")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
            ]
            .into_iter()
            .chain(additional_projects)
            .collect(),
            BsonAutoId::ObjectId,
        )
        .unwrap();
    litedb.serialize()
}

fn defined_projects_in_litedb(env_projects_str: &str) -> Vec<String> {
    [
        format!("{env_projects_str}/HistoryOfAvatarOptimizer"),
        format!("{env_projects_str}/VPMPackageAutoInstaller"),
        format!("{env_projects_str}/CrashOnExitWithLogTypeFullName"),
        format!("{env_projects_str}/New Project 32"),
        format!("{env_projects_str}/New Project 31"),
    ]
    .into_iter()
    .sorted()
    .map(normalize_path)
    .collect_vec()
}

fn load_projects_in_litedb(litedb: &[u8]) -> Vec<String> {
    LiteDBFile::parse(litedb)
        .unwrap()
        .get_all("projects")
        .map(|p| p["Path"].as_str().unwrap().to_string())
        .map(normalize_path)
        .sorted()
        .collect_vec()
}

fn load_projects_in_sqlite(path: PathBuf) -> Vec<String> {
    rusqlite::Connection::open(path)
        .unwrap()
        .prepare("SELECT path FROM projects ORDER BY path")
        .unwrap()
        .query(())
        .unwrap()
        .map(|x| x.get::<_, String>(0))
        .collect::<Vec<_>>()
        .unwrap()
}

/// Migrate from settings.json => vcc.liteDb for VCC 2.1.x or older compatibility
#[tokio::test]
async fn load_no_litedb_environment() {
    // initialize env
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(env_projects_str),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
    }
}

/// If there are both settings.json and vcc.liteDb, settings.json is the origin of data.
/// This is for compatibility for legacy vpm cli and other tools that edits settings.json.
/// Tools that recognizes vcc.liteDb should also update settings.json so no problems are there
#[tokio::test]
async fn both_litedb_and_settings() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(env_projects_str),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(env_projects_str),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_settings_json!(env_projects_str),
        );
    }
}

/// When the settings.json does not have `userProjects` key,
/// ALCOM assumes that the data is migrated to no-settings.json-era.
/// We use vcc.litedb as a origin of trust.
/// For compatibility with older tools and manually configured settings.json,
/// we generally copy data of vcc.litedb to settings.json.
///
/// This test tests 1) and 2) behavior
#[tokio::test]
async fn no_project_data_in_settings_json() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(env_dir.join(SETTINGS_JSON), "{}").unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(env_projects_str),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_litedb(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_litedb(env_projects_str),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_litedb(env_projects_str),
        );
    }
}

/// When no settings.json is there, we treat empty userProjects are there.
/// This behavior is dissuading, but I hope VCC will never remove settings.json so no problem in the future.
#[tokio::test]
async fn no_settings_json() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(env_projects_str),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_litedb(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_litedb(env_projects_str),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_litedb(env_projects_str),
        );
    }
}

/// If there is a relative path in settings.json, there is no stable behavior for it.
///
/// vrc-get decides to not support relative paths in settings.json and exclude them for migration.
/// As a result, relative paths are removed from settings.json.
#[tokio::test]
async fn relative_path_in_settings_json_and_database() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec!["relative-path-here".into()],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(
            env_projects_str,
            additional_projects = vec![vrc_get_litedb::document! {
                "Path" => "relative-path-here",
                "UnityVersion" => "2022.3.22f1",
                "CreatedAt" => DateTime::now(),
                "LastModified" => DateTime::now(),
                "Type" => ProjectType::Avatars as i32,
                "Favorite" => false,
            },]
        ),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_settings_json!(env_projects_str),
        );
    }
}

/// vrc-get considers projects with non-objectid _id as invalid.
#[tokio::test]
async fn invalid_object_id() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec![format!("{env_projects_str}/bad id")],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(
            env_projects_str,
            additional_projects = vec![vrc_get_litedb::document! {
                "_id" => "_",
                "Path" => format!("{env_projects_str}/bad id"),
                "UnityVersion" => "2022.3.22f1",
                "CreatedAt" => DateTime::now(),
                "LastModified" => DateTime::now(),
                "Type" => ProjectType::Avatars as i32,
                "Favorite" => false,
            },]
        ),
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_settings_json!(env_projects_str),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_settings_json!(env_projects_str),
        );
    }
}

/// vrc-get considers projects with non-objectid _id as invalid.
#[tokio::test]
async fn migrate_with_real_project() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec![format!("{env_projects_str}/real-project")],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(env_projects_str),
    )
    .unwrap();
    // create fake project
    std::fs::create_dir_all(format!("{env_projects_str}/real-project")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/Assets")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/Packages")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/ProjectSettings")).unwrap();
    std::fs::write(
        format!("{env_projects_str}/real-project/ProjectSettings/ProjectVersion.txt"),
        b"m_EditorVersion: 2019.4.31f1
m_EditorVersionWithRevision: 2019.4.31f1 (bd5abf232a62)
",
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(
                env_projects_str,
                additional_projects = [format!("{env_projects_str}/real-project")],
            ),
        );
        assert_eq!(
            load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
            defined_projects_in_settings_json!(
                env_projects_str,
                additional_projects = [format!("{env_projects_str}/real-project")],
            ),
        );
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_settings_json!(
                env_projects_str,
                additional_projects = [format!("{env_projects_str}/real-project")],
            ),
        );
        assert_eq!(
            LiteDBFile::parse(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap())
                .unwrap()
                .get_by_index(
                    "projects",
                    "Path",
                    &normalize_path(format!("{env_projects_str}/real-project")).into()
                )
                .next()
                .unwrap()["UnityVersion"],
            vrc_get_litedb::bson::Value::String("2019.4.31f1".into())
        );
    }
}

#[tokio::test]
async fn sqlite_migrations_new_projects() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec![
                format!("{env_projects_str}/real-project"),
                format!("{env_projects_str}/litedb-extended-project"),
            ],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(
            env_projects_str,
            additional_projects = vec![vrc_get_litedb::document! {
                "_id" => object_id(b"extendedproj"),
                "Path" => format!("{env_projects_str}/litedb-extended-project"),
                "UnityVersion" => "2022.3.22f1",
                "CreatedAt" => vrc_get_litedb::date!(2022-03-02 11:02:14),
                "LastModified" => vrc_get_litedb::date!(2024-03-02 12:15:00),
                "Type" => ProjectType::Avatars as i32,
                "Favorite" => true,
                "vrc-get" => vrc_get_litedb::document! {
                    "cached_unity_version" => "2022.3.22f1",
                    "unity_revision" => "887be4894c44",
                    "custom_unity_args" => vrc_get_litedb::array!["-batchmode"],
                    "unity_path" => "/Applications/Unity.app/Contents/MacOS/Unity",
                    "is_valid" => false,
                },
            }]
        ),
    )
    .unwrap();
    // create fake project
    std::fs::create_dir_all(format!("{env_projects_str}/real-project")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/Assets")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/Packages")).unwrap();
    std::fs::create_dir_all(format!("{env_projects_str}/real-project/ProjectSettings")).unwrap();
    std::fs::write(
        format!("{env_projects_str}/real-project/ProjectSettings/ProjectVersion.txt"),
        b"m_EditorVersion: 2019.4.31f1
m_EditorVersionWithRevision: 2019.4.31f1 (bd5abf232a62)
",
    )
    .unwrap();

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        // run code
        let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
        let manage = ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            defined_projects_in_settings_json!(
                env_projects_str,
                additional_projects = [
                    format!("{env_projects_str}/real-project"),
                    format!("{env_projects_str}/litedb-extended-project")
                ],
            ),
        );
        let conn = rusqlite::Connection::open(env_dir.join(VRC_GET_SQLITE)).unwrap();
        let get_project_version = |path: &str| {
            conn.query_one(
                "SELECT unity_version_with_revision FROM projects WHERE path = ?",
                [normalize_path(path.into())],
                |x| x.get::<_, String>(0),
            )
            .unwrap()
        };

        // project listed in settings.json and not in litedb, and exists in fs: load data from fs
        assert_eq!(
            get_project_version(&format!("{env_projects_str}/real-project")),
            "2019.4.31f1".to_string(),
        );
        let real_project = manage
            .find_project(&format!("{env_projects_str}/real-project"))
            .unwrap()
            .unwrap();
        assert_eq!(
            real_project.unity_version(),
            Some(UnityVersion::new_f1(2019, 4, 31))
        );
        assert_eq!(real_project.unity_revision(), None);
        assert_eq!(
            real_project.path(),
            &normalize_path(format!("{env_projects_str}/real-project"))
        );
        assert_eq!(real_project.name(), "real-project");

        // project listed in litedb, and not exists in fs: copy from litedb
        assert_eq!(
            get_project_version(&format!("{env_projects_str}/HistoryOfAvatarOptimizer")),
            "2022.3.22f1".to_string(),
        );
        let real_project = manage
            .find_project(&format!("{env_projects_str}/HistoryOfAvatarOptimizer"))
            .unwrap()
            .unwrap();
        assert_eq!(
            real_project.unity_version(),
            Some(UnityVersion::new_f1(2022, 3, 22))
        );
        assert_eq!(real_project.unity_revision(), None);

        // project listed in litedb, and not exists in fs: copy from litedb, with vrc-get extensions
        assert_eq!(
            get_project_version(&format!("{env_projects_str}/litedb-extended-project")),
            "2022.3.22f1(887be4894c44)".to_string(),
        );
        let real_project = manage
            .find_project(&format!("{env_projects_str}/litedb-extended-project"))
            .unwrap()
            .unwrap();
        assert_eq!(
            real_project.path(),
            &normalize_path(format!("{env_projects_str}/litedb-extended-project"))
        );
        assert_eq!(real_project.name(), "litedb-extended-project");
        assert_eq!(
            real_project.litedb_objectid(),
            Some(object_id(b"extendedproj"))
        );
        assert_eq!(
            real_project.crated_at(),
            vrc_get_litedb::date!(2022-03-02 11:02:14)
        );
        assert_eq!(
            real_project.last_modified(),
            vrc_get_litedb::date!(2024-03-02 12:15:00)
        );
        assert_eq!(
            real_project.unity_version(),
            Some(UnityVersion::new_f1(2022, 3, 22))
        );
        assert_eq!(real_project.unity_revision(), Some("887be4894c44"));
        assert_eq!(real_project.project_type(), ProjectType::Avatars);
        assert!(real_project.favorite());
        assert_eq!(
            real_project.custom_unity_args(),
            Some(&["-batchmode".to_string()][..])
        );
        assert_eq!(
            real_project.unity_path(),
            Some("/Applications/Unity.app/Contents/MacOS/Unity")
        );
        assert!(!real_project.is_valid_project());
    }
}

async fn sqlite_migrations_with_projects_in_database_prepare(
    env_dir: &Path,
    env_projects_str: &str,
) {
    clean_dir(env_dir);
    {
        // Create tables
        let io = &DefaultEnvironmentIo::new(env_dir.into());
        ProjectManagement::start(io).await.unwrap();

        std::fs::create_dir_all(env_dir.join(VRC_GET_SQLITE).parent().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(env_dir.join(VRC_GET_SQLITE)).unwrap();
        let mut stmt = conn
            .prepare(
                "INSERT INTO projects \
                    (path, litedb_objectid, unity_version_with_revision, \
                    created_at, last_modified, type, favorite, is_valid)\
                VALUES (?, ?, ?, unixepoch('now'), unixepoch('now'), 0, 0, 1) ",
            )
            .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/sqlite-only-no-id")),
            Option::<String>::None,
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/sqlite-only-with-id")),
            object_id(b"sqlite-only ").as_bytes().encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/id-mismatch")),
            object_id(b"id-mismatch1").as_bytes().encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"
            )),
            object_id(b"pathmismatc0").as_bytes().encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/path-mismatch-1-litedb-is-newer-sqlite"
            )),
            object_id(b"pathmismatc1").as_bytes().encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/both-conflict-0-path-conflicter"
            )),
            object_id(b"random014752").as_bytes().encode_hex::<String>(),
            "2019.4.31f1(bd5abf232a62)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/both-conflict-0-id-conflicter")),
            object_id(b"bothconflic0").as_bytes().encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();

        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/both-conflict-1-path-conflicter"
            )),
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"randomo23026")
                .as_bytes()
                .encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/both-conflict-1-id-conflicter")),
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"bothconflic1")
                .as_bytes()
                .encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
    }
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec![
                format!("{env_projects_str}/litedb-good"),
                format!("{env_projects_str}/id-mismatch"),
                format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-litedb"),
                format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
                format!("{env_projects_str}/both-conflict-0-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-id-conflicter"),
            ],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(
            env_projects_str,
            additional_projects = vec![
                vrc_get_litedb::document! {
                    "_id" => object_id(b"litedbgoodpr"),
                    "Path" => normalize_path(format!("{env_projects_str}/litedb-good")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-02 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-02 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => object_id(b"id-mismatch0"),
                    "Path" => normalize_path(format!("{env_projects_str}/id-mismatch")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-12 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-03 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => object_id(b"pathmismatc0"),
                    "Path" => normalize_path(format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-litedb")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-12 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-03 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => object_id(b"pathmismatc1"),
                    "Path" => normalize_path(format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now().add_days(1).unwrap(),
                    "LastModified" => DateTime::now().add_days(1).unwrap(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => object_id(b"bothconflic0"),
                    "Path" => normalize_path(format!("{env_projects_str}/both-conflict-0-path-conflicter")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now().add_days(1).unwrap(),
                    "LastModified" => DateTime::now().add_days(1).unwrap(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"bothconflic1"),
                    "Path" => normalize_path(format!("{env_projects_str}/both-conflict-1-path-conflicter")),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now().add_days(1).unwrap(),
                    "LastModified" => DateTime::now().add_days(1).unwrap(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
            ]
        ),
    )
    .unwrap();
}

fn sorted<T: Ord>(mut value: Vec<T>) -> Vec<T> {
    value.sort();
    value
}

#[tokio::test]
async fn sqlite_litedb_sync() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    sqlite_migrations_with_projects_in_database_prepare(&env_dir, env_projects_str).await;
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());

    for iteration in 0..2 {
        eprintln!("initial migration iteration: {iteration}");
        // run code
        let manage = ProjectManagement::start(io).await.unwrap(); // does migration

        // check data
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            sorted(vec![
                normalize_path(format!("{env_projects_str}/sqlite-only-no-id")),
                normalize_path(format!("{env_projects_str}/sqlite-only-with-id")),
                normalize_path(format!("{env_projects_str}/id-mismatch")),
                normalize_path(format!(
                    "{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"
                )),
                normalize_path(format!(
                    "{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"
                )),
                normalize_path(format!(
                    "{env_projects_str}/both-conflict-0-path-conflicter"
                )),
                normalize_path(format!("{env_projects_str}/both-conflict-0-id-conflicter")),
                normalize_path(format!(
                    "{env_projects_str}/both-conflict-1-path-conflicter"
                )),
                normalize_path(format!("{env_projects_str}/both-conflict-1-id-conflicter")),
            ]),
        );

        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            defined_projects_in_settings_json!(
                env_projects_str,
                additional_projects = [
                    format!("{env_projects_str}/litedb-good"),
                    format!("{env_projects_str}/id-mismatch"),
                    format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"),
                    format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
                    format!("{env_projects_str}/both-conflict-0-path-conflicter"),
                    format!("{env_projects_str}/both-conflict-1-path-conflicter"),
                    format!("{env_projects_str}/both-conflict-1-id-conflicter"),
                ],
            ),
        );

        assert_eq!(
            sorted(manage.find_project_list_mismatch().await.unwrap()),
            sorted(vec![
                // shared
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!("{env_projects_str}/Blank 2019 project")),
                },
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!("{env_projects_str}/Blank 2022 project")),
                },
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!(
                        "{env_projects_str}/CrashOnExitWithLogTypeFullName"
                    )),
                },
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!("{env_projects_str}/HistoryOfAvatarOptimizer")),
                },
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!("{env_projects_str}/VPMPackageAutoInstaller")),
                },
                // our tests
                ProjectListMismatch::OnlyInVcc {
                    path: normalize_path(format!("{env_projects_str}/litedb-good")),
                },
                // sqlite-only-no-id has never been live in LiteDB, so it's not a mismatch
                ProjectListMismatch::OnlyInVrcGet {
                    path: normalize_path(format!("{env_projects_str}/sqlite-only-with-id")),
                },
                ProjectListMismatch::OnlyInVrcGet {
                    path: normalize_path(format!(
                        "{env_projects_str}/both-conflict-0-id-conflicter"
                    )),
                },
            ]),
        );

        let get_project = |name: &str| {
            manage
                .find_project(&format!("{env_projects_str}/{name}"))
                .unwrap()
        };

        let id_mismatch = get_project("id-mismatch").unwrap();
        assert_eq!(
            id_mismatch.litedb_objectid(),
            Some(object_id(b"id-mismatch0"))
        );

        assert!(
            get_project("litedb-good").is_none(),
            "litedb-good should not be added to sqlite yet"
        );

        let path_mismatch_0 = get_project("path-mismatch-0-sqlite-is-newer-sqlite").unwrap();
        assert_eq!(
            path_mismatch_0.litedb_objectid(),
            Some(object_id(b"pathmismatc0"))
        );
        assert!(get_project("path-mismatch-0-sqlite-is-newer-litedb").is_none());

        let path_mismatch_1 = get_project("path-mismatch-1-litedb-is-newer-litedb").unwrap();
        assert_eq!(
            path_mismatch_1.litedb_objectid(),
            Some(object_id(b"pathmismatc1"))
        );
        assert!(get_project("path-mismatch-1-litedb-is-newer-sqlite").is_none());

        let conflict_0_path_conflicter = get_project("both-conflict-0-path-conflicter").unwrap();
        assert_eq!(
            conflict_0_path_conflicter.litedb_objectid(),
            Some(object_id(b"bothconflic0"))
        );
        // LiteDB does not have revision, but the release is the same as SQLite one,
        // so the revision in SQLite is kept.
        assert_eq!(
            conflict_0_path_conflicter.unity_revision(),
            Some("887be4894c44")
        );
    }

    {
        let mut manage = ProjectManagement::start(io).await.unwrap();

        manage
            .fix_project_list_mismatch(
                &[
                    &normalize_path(format!("{env_projects_str}/Blank 2019 project")),
                    &normalize_path(format!("{env_projects_str}/Blank 2022 project")),
                    &normalize_path(format!("{env_projects_str}/CrashOnExitWithLogTypeFullName")),
                    &normalize_path(format!("{env_projects_str}/HistoryOfAvatarOptimizer")),
                    &normalize_path(format!("{env_projects_str}/VPMPackageAutoInstaller")),
                    //
                    &normalize_path(format!("{env_projects_str}/litedb-good")),
                ],
                &[&normalize_path(format!(
                    "{env_projects_str}/sqlite-only-with-id"
                ))],
                &[],
                &[&normalize_path(format!(
                    "{env_projects_str}/both-conflict-0-id-conflicter"
                ))],
            )
            .await
            .unwrap();
    }

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        let manage = ProjectManagement::start(io).await.unwrap(); // does migration

        let litedb_project_list = defined_projects_in_settings_json!(
            env_projects_str,
            additional_projects = [
                format!("{env_projects_str}/litedb-good"),
                format!("{env_projects_str}/sqlite-only-with-id"),
                format!("{env_projects_str}/id-mismatch"),
                format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"),
                format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
                format!("{env_projects_str}/both-conflict-0-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-id-conflicter"),
            ],
        );
        // sqlite-only-no-id is vrc-get only project so it's not in LiteDB
        let sqlite_project_list = sorted(
            (litedb_project_list.iter().cloned())
                .chain([normalize_path(format!(
                    "{env_projects_str}/sqlite-only-no-id"
                ))])
                .collect(),
        );

        // check data
        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            sqlite_project_list,
        );

        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            litedb_project_list,
        );

        let get_project = |name: &str| {
            manage
                .find_project(&format!("{env_projects_str}/{name}"))
                .unwrap()
        };

        let sqlite_only_with_id = get_project("sqlite-only-with-id").unwrap();
        assert_eq!(
            sqlite_only_with_id.litedb_objectid(),
            Some(object_id(b"sqlite-only "))
        );

        let sqlite_only_no_id = get_project("sqlite-only-no-id").unwrap();
        assert_eq!(sqlite_only_no_id.litedb_objectid(), None);

        assert!(get_project("both-conflict-0-id-conflicter").is_none());
    }

    {
        let manage = ProjectManagement::start(io).await.unwrap(); // does migration
        assert_eq!(
            sorted(manage.find_project_list_mismatch().await.unwrap()),
            vec![]
        );
    }
}

/// Projects without litedb_objectid have never been live in LiteDB, so they're vrc-get only
/// projects rather than a mismatch. Automatic sync modes must neither remove them from SQLite
/// nor add them to LiteDB.
///
/// `sqlite_only_with_id_kept`: whether projects once live in LiteDB but no longer exist there
/// are restored (ProjectsUnion) or removed (TrustLitedb).
async fn sqlite_litedb_sync_auto_mode(
    env_dir: PathBuf,
    env_projects: PathBuf,
    mode: &str,
    sqlite_only_with_id_kept: bool,
) {
    common::init_log();
    let env_projects_str = env_projects.to_str().unwrap();
    sqlite_migrations_with_projects_in_database_prepare(&env_dir, env_projects_str).await;
    std::fs::write(
        env_dir.join(VRC_GET_SETTINGS),
        format!(r#"{{"projectListSyncMode": "{mode}"}}"#),
    )
    .unwrap();
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());

    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        let manage = ProjectManagement::start(io).await.unwrap(); // does migration

        let mut litedb_project_list = defined_projects_in_settings_json!(
            env_projects_str,
            additional_projects = [
                format!("{env_projects_str}/litedb-good"),
                format!("{env_projects_str}/id-mismatch"),
                format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"),
                format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
                format!("{env_projects_str}/both-conflict-0-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-path-conflicter"),
                format!("{env_projects_str}/both-conflict-1-id-conflicter"),
            ],
        );
        if sqlite_only_with_id_kept {
            litedb_project_list.extend([
                normalize_path(format!("{env_projects_str}/sqlite-only-with-id")),
                normalize_path(format!("{env_projects_str}/both-conflict-0-id-conflicter")),
            ]);
            litedb_project_list.sort();
        }
        let sqlite_project_list = sorted(
            (litedb_project_list.iter().cloned())
                .chain([normalize_path(format!(
                    "{env_projects_str}/sqlite-only-no-id"
                ))])
                .collect(),
        );

        assert_eq!(
            load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
            sqlite_project_list,
        );
        assert_eq!(
            load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
            litedb_project_list,
        );
        assert_eq!(
            sorted(manage.find_project_list_mismatch().await.unwrap()),
            vec![]
        );

        let get_project = |name: &str| {
            manage
                .find_project(&format!("{env_projects_str}/{name}"))
                .unwrap()
        };

        // never been in LiteDB: kept as vrc-get only project
        let sqlite_only_no_id = get_project("sqlite-only-no-id").unwrap();
        assert_eq!(sqlite_only_no_id.litedb_objectid(), None);

        // once been in LiteDB but no longer exists
        assert_eq!(
            get_project("sqlite-only-with-id").is_some(),
            sqlite_only_with_id_kept
        );
        assert_eq!(
            get_project("both-conflict-0-id-conflicter").is_some(),
            sqlite_only_with_id_kept
        );
    }
}

#[tokio::test]
async fn sqlite_litedb_sync_trust_litedb() {
    sqlite_litedb_sync_auto_mode(
        get_temp_path("environment"),
        get_temp_path("env_projects"),
        "TrustLitedb",
        false,
    )
    .await;
}

#[tokio::test]
async fn sqlite_litedb_sync_projects_union() {
    sqlite_litedb_sync_auto_mode(
        get_temp_path("environment"),
        get_temp_path("env_projects"),
        "ProjectsUnion",
        true,
    )
    .await;
}

/// Tests for how metadata is merged when a project exists on both SQLite and LiteDB.
///
/// - the values from the database with newer last_modified are used
/// - if last_modified is the same, SQLite (ALCOM) values are preferred
/// - for unity version, if the release is the same, the one with revision is used regardless of last_modified
#[tokio::test]
async fn sqlite_litedb_sync_metadata() {
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());

    let old_created = vrc_get_litedb::date!(2022-03-02 11:02:14);
    let new_created = vrc_get_litedb::date!(2023-03-02 11:02:14);
    let old_modified = vrc_get_litedb::date!(2024-03-02 12:15:00);
    let new_modified = vrc_get_litedb::date!(2024-06-02 12:15:00);

    let unix = |date: DateTime| date.as_unix_milliseconds() / 1000;
    let path = |name: &str| normalize_path(format!("{env_projects_str}/{name}"));

    const REVISION_2019: &str = "bd5abf232a62";
    const REVISION_2022: &str = "887be4894c44";

    clean_dir(&env_dir);
    {
        // Create tables
        ProjectManagement::start(io).await.unwrap();

        // In SQLite, all projects are non-favorite worlds project created at new_created
        let conn = rusqlite::Connection::open(env_dir.join(VRC_GET_SQLITE)).unwrap();
        let mut stmt = conn
            .prepare(
                "INSERT INTO projects \
                    (path, litedb_objectid, unity_version_with_revision, \
                    created_at, last_modified, type, favorite, is_valid)\
                VALUES (?, ?, ?, ?, ?, ?, 0, 1) ",
            )
            .unwrap();
        let mut insert = |name: &str, id: &[u8; 12], unity: &str, last_modified: DateTime| {
            stmt.execute((
                path(name),
                object_id(id).as_bytes().encode_hex::<String>(),
                unity,
                unix(new_created),
                unix(last_modified),
                ProjectType::Worlds as i32,
            ))
            .unwrap();
        };
        insert(
            "litedb-is-newer",
            b"litedb-newer",
            "2019.4.31f1(bd5abf232a62)",
            old_modified,
        );
        insert(
            "sqlite-is-newer",
            b"sqlite-newer",
            "2019.4.31f1(bd5abf232a62)",
            new_modified,
        );
        insert(
            "same-modified",
            b"samemodified",
            "2019.4.31f1(bd5abf232a62)",
            old_modified,
        );
        insert(
            "same-release-litedb-is-newer",
            b"same-rel-lit",
            "2022.3.22f1(887be4894c44)",
            old_modified,
        );
        insert(
            "same-release-sqlite-is-newer",
            b"same-rel-sql",
            "2022.3.22f1",
            new_modified,
        );
    }

    // In LiteDB, all projects are favorite avatars project created at old_created
    let litedb_project = |name: &str, id: &[u8; 12], last_modified: DateTime| {
        vrc_get_litedb::document! {
            "_id" => object_id(id),
            "Path" => path(name),
            "UnityVersion" => "2022.3.22f1",
            "CreatedAt" => old_created,
            "LastModified" => last_modified,
            "Type" => ProjectType::Avatars as i32,
            "Favorite" => true,
        }
    };
    let mut same_release_sqlite_is_newer = litedb_project(
        "same-release-sqlite-is-newer",
        b"same-rel-sql",
        old_modified,
    );
    same_release_sqlite_is_newer.insert(
        "vrc-get",
        vrc_get_litedb::document! {
            "cached_unity_version" => "2022.3.22f1",
            "unity_revision" => REVISION_2022,
        },
    );
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects!(
            env_projects_str,
            additional_projects = vec![
                format!("{env_projects_str}/litedb-is-newer"),
                format!("{env_projects_str}/sqlite-is-newer"),
                format!("{env_projects_str}/same-modified"),
                format!("{env_projects_str}/same-release-litedb-is-newer"),
                format!("{env_projects_str}/same-release-sqlite-is-newer"),
            ],
        ),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects!(
            env_projects_str,
            additional_projects = vec![
                litedb_project("litedb-is-newer", b"litedb-newer", new_modified),
                litedb_project("sqlite-is-newer", b"sqlite-newer", old_modified),
                litedb_project("same-modified", b"samemodified", old_modified),
                litedb_project(
                    "same-release-litedb-is-newer",
                    b"same-rel-lit",
                    new_modified
                ),
                same_release_sqlite_is_newer,
            ]
        ),
    )
    .unwrap();

    // The result must be stable, so we run the sync twice.
    for iteration in 0..2 {
        eprintln!("iteration: {iteration}");
        let manage = ProjectManagement::start(io).await.unwrap();

        let get_project = |name: &str| {
            manage
                .find_project(&format!("{env_projects_str}/{name}"))
                .unwrap()
                .unwrap()
        };

        // LiteDB is newer: everything comes from LiteDB
        let project = get_project("litedb-is-newer");
        assert_eq!(project.crated_at(), old_created);
        assert_eq!(project.last_modified(), new_modified);
        assert_eq!(project.project_type(), ProjectType::Avatars);
        assert!(project.favorite());
        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new_f1(2022, 3, 22))
        );
        assert_eq!(project.unity_revision(), None);

        // SQLite is newer: everything is kept
        let project = get_project("sqlite-is-newer");
        assert_eq!(project.crated_at(), new_created);
        assert_eq!(project.last_modified(), new_modified);
        assert_eq!(project.project_type(), ProjectType::Worlds);
        assert!(!project.favorite());
        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new_f1(2019, 4, 31))
        );
        assert_eq!(project.unity_revision(), Some(REVISION_2019));

        // last_modified is the same: SQLite is preferred, so everything is kept
        let project = get_project("same-modified");
        assert_eq!(project.crated_at(), new_created);
        assert_eq!(project.last_modified(), old_modified);
        assert_eq!(project.project_type(), ProjectType::Worlds);
        assert!(!project.favorite());
        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new_f1(2019, 4, 31))
        );
        assert_eq!(project.unity_revision(), Some(REVISION_2019));

        // LiteDB is newer, but the release is the same and only SQLite has revision:
        // revision in SQLite is kept, and others come from LiteDB
        let project = get_project("same-release-litedb-is-newer");
        assert_eq!(project.crated_at(), old_created);
        assert_eq!(project.last_modified(), new_modified);
        assert_eq!(project.project_type(), ProjectType::Avatars);
        assert!(project.favorite());
        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new_f1(2022, 3, 22))
        );
        assert_eq!(project.unity_revision(), Some(REVISION_2022));

        // SQLite is newer, but the release is the same and only LiteDB has revision:
        // revision comes from LiteDB, and others are kept
        let project = get_project("same-release-sqlite-is-newer");
        assert_eq!(project.crated_at(), new_created);
        assert_eq!(project.last_modified(), new_modified);
        assert_eq!(project.project_type(), ProjectType::Worlds);
        assert!(!project.favorite());
        assert_eq!(
            project.unity_version(),
            Some(UnityVersion::new_f1(2022, 3, 22))
        );
        assert_eq!(project.unity_revision(), Some(REVISION_2022));
    }
}
