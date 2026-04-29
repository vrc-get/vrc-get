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

#![cfg(feature = "experimental-project-management")]

use crate::common::get_temp_path;
use itertools::Itertools;
use std::path::Path;
use vrc_get_litedb::bson::DateTime;
use vrc_get_litedb::file_io::{BsonAutoId, LiteDBFile};
use vrc_get_vpm::ProjectType;
use vrc_get_vpm::environment::{Settings, VccDatabaseConnection};
use vrc_get_vpm::io::DefaultEnvironmentIo;

mod common;

fn clean_dir(path: &Path) {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Ok(()) => {}
        Err(e) => panic!("error cleaning dir {}: {}", path.display(), e),
    }
    std::fs::create_dir_all(path).unwrap();
}

const VCC_LITEDB: &str = "vcc.liteDb";
const SETTINGS_JSON: &str = "settings.json";

fn test_settings_json_with_projects(env_projects_str: &str) -> String {
    format!(
        r#"{{
  "userProjects": [
    "{env_projects_str}/Blank 2019 project",
    "{env_projects_str}/Blank 2022 project",
    "{env_projects_str}/HistoryOfAvatarOptimizer",
    "{env_projects_str}/VPMPackageAutoInstaller",
    "{env_projects_str}/CrashOnExitWithLogTypeFullName"
  ]
}}"#
    )
}

fn defined_projects_in_settings_json(env_projects_str: &str) -> Vec<String> {
    [
        format!("{env_projects_str}/Blank 2019 project"),
        format!("{env_projects_str}/Blank 2022 project"),
        format!("{env_projects_str}/HistoryOfAvatarOptimizer"),
        format!("{env_projects_str}/VPMPackageAutoInstaller"),
        format!("{env_projects_str}/CrashOnExitWithLogTypeFullName"),
    ]
    .into_iter()
    .sorted()
    .collect_vec()
}

fn load_projects_in_settings_json(settings_json: &[u8]) -> Vec<String> {
    serde_json::from_slice::<serde_json::Value>(settings_json).unwrap()["userProjects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_string())
        .sorted()
        .collect_vec()
}

fn test_litedb_file_with_projects(env_projects_str: &str) -> Vec<u8> {
    let mut litedb = LiteDBFile::new();
    litedb
        .insert(
            "projects",
            vec![
                // "Blank 2019 project" and "Blank 2022 project" are not exist in litedb
                // those three are exist on both litedb and settings.json
                vrc_get_litedb::document! {
                    "Path" => format!("{env_projects_str}/HistoryOfAvatarOptimizer"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => format!("{env_projects_str}/VPMPackageAutoInstaller"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => format!("{env_projects_str}/CrashOnExitWithLogTypeFullName"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                // Those two projects are exist only on litedb
                vrc_get_litedb::document! {
                    "Path" => format!("{env_projects_str}/New Project 32"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
                vrc_get_litedb::document! {
                    "Path" => format!("{env_projects_str}/New Project 31"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => DateTime::now(),
                    "LastModified" => DateTime::now(),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => false,
                },
            ],
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
    .collect_vec()
}

fn load_projects_in_litedb(litedb: &[u8]) -> Vec<String> {
    LiteDBFile::parse(litedb)
        .unwrap()
        .get_all("projects")
        .map(|p| p["Path"].as_str().unwrap().to_string())
        .sorted()
        .collect_vec()
}

/// Migrate from settings.json => vcc.liteDb for VCC 2.1.x or older compatibility
#[tokio::test]
async fn load_no_litedb_environment() {
    // initialize env
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects(env_projects_str),
    )
    .unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let mut settings = Settings::load(io).await.unwrap();
    let mut connection = VccDatabaseConnection::connect(io).await.unwrap();
    connection.migrate(&settings, io).await.unwrap();
    connection.dedup_projects();
    connection.normalize_path();
    connection.save(io).await.unwrap();
    settings.load_from_db(&connection).unwrap();
    settings.save(io).await.unwrap();

    // check
    assert_eq!(
        load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
        defined_projects_in_settings_json(env_projects_str),
    );
    assert_eq!(
        load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
        defined_projects_in_settings_json(env_projects_str),
    );
}

/// If there are both settings.json and vcc.liteDb, settings.json is the origin of data.
/// This is for compatibility for legacy vpm cli and other tools that edits settings.json.
/// Tools that recognizes vcc.liteDb should also update settings.json so no problems are there
#[tokio::test]
async fn both_litedb_and_settings() {
    // initialize environment
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(SETTINGS_JSON),
        test_settings_json_with_projects(env_projects_str),
    )
    .unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects(env_projects_str),
    )
    .unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let mut settings = Settings::load(io).await.unwrap();
    let mut connection = VccDatabaseConnection::connect(io).await.unwrap();
    connection.migrate(&settings, io).await.unwrap();
    connection.dedup_projects();
    connection.normalize_path();
    connection.save(io).await.unwrap();
    settings.load_from_db(&connection).unwrap();
    settings.save(io).await.unwrap();

    // check data
    assert_eq!(
        load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
        defined_projects_in_settings_json(env_projects_str),
    );
    assert_eq!(
        load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
        defined_projects_in_settings_json(env_projects_str),
    );
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
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(env_dir.join(SETTINGS_JSON), "{}").unwrap();
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects(env_projects_str),
    )
    .unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let mut settings = Settings::load(io).await.unwrap();
    let mut connection = VccDatabaseConnection::connect(io).await.unwrap();
    connection.migrate(&settings, io).await.unwrap();
    connection.dedup_projects();
    connection.normalize_path();
    connection.save(io).await.unwrap();
    settings.load_from_db(&connection).unwrap();
    settings.save(io).await.unwrap();

    // check data
    assert_eq!(
        load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
        defined_projects_in_litedb(env_projects_str),
    );
    assert_eq!(
        load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
        defined_projects_in_litedb(env_projects_str),
    );
}

/// When no settings.json is there, we treat empty userProjects are there.
/// This behavior is dissuading, but I hope VCC will never remove settings.json so no problem in the future.
#[tokio::test]
async fn no_settings_json() {
    // initialize environment
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    clean_dir(&env_dir);
    std::fs::write(
        env_dir.join(VCC_LITEDB),
        test_litedb_file_with_projects(env_projects_str),
    )
    .unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let mut settings = Settings::load(io).await.unwrap();
    let mut connection = VccDatabaseConnection::connect(io).await.unwrap();
    connection.migrate(&settings, io).await.unwrap();
    connection.dedup_projects();
    connection.normalize_path();
    connection.save(io).await.unwrap();
    settings.load_from_db(&connection).unwrap();
    settings.save(io).await.unwrap();

    // check data
    assert_eq!(
        load_projects_in_litedb(&std::fs::read(env_dir.join(VCC_LITEDB)).unwrap()),
        vec![] as Vec<String>,
    );
    assert_eq!(
        load_projects_in_settings_json(&std::fs::read(env_dir.join(SETTINGS_JSON)).unwrap()),
        vec![] as Vec<String>,
    );
}
