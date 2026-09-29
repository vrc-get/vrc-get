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
use hex::ToHex;
use itertools::Itertools;
use rusqlite::fallible_iterator::FallibleIterator;
use std::path::{Path, PathBuf};
use vrc_get_litedb::bson::DateTime;
use vrc_get_litedb::file_io::{BsonAutoId, LiteDBFile};
use vrc_get_vpm::ProjectType;
use vrc_get_vpm::environment::ProjectManagement;
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
        ]).collect::<Vec<_>>()
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
        .sorted()
        .map(normalize_path)
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
                &format!("{env_projects_str}/real-project").into()
            )
            .next()
            .unwrap()["UnityVersion"],
        vrc_get_litedb::bson::Value::String("2019.4.31f1".into())
    );
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
                "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"extendedproj"),
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
            [path],
            |x| x.get::<_, String>(0),
        )
        .unwrap()
    };

    // project listed in settings.json and not in litedb, and exists in fs: load data from fs
    assert_eq!(
        get_project_version(&format!("{env_projects_str}/real-project")),
        "2019.4.31f1(bd5abf232a62)".to_string(),
    );
    let real_project = manage
        .find_project(&format!("{env_projects_str}/real-project"))
        .unwrap()
        .unwrap();
    assert_eq!(
        real_project.unity_version(),
        Some(UnityVersion::new_f1(2019, 4, 31))
    );
    assert_eq!(real_project.unity_revision(), Some("bd5abf232a62"));
    assert_eq!(
        real_project.path(),
        &format!("{env_projects_str}/real-project")
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
        &format!("{env_projects_str}/litedb-extended-project")
    );
    assert_eq!(real_project.name(), "litedb-extended-project");
    assert_eq!(
        real_project.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"extendedproj"))
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
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"sqlite-only ")
                .as_bytes()
                .encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!("{env_projects_str}/id-mismatch")),
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"id-mismatch1")
                .as_bytes()
                .encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"
            )),
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc0")
                .as_bytes()
                .encode_hex::<String>(),
            "2022.3.22f1(887be4894c44)",
        ))
        .unwrap();
        stmt.execute((
            normalize_path(format!(
                "{env_projects_str}/path-mismatch-1-litedb-is-newer-sqlite"
            )),
            vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc1")
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
                    "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"litedbgoodpr"),
                    "Path" => format!("{env_projects_str}/litedb-good"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-02 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-02 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"id-mismatch0"),
                    "Path" => format!("{env_projects_str}/id-mismatch"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-12 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-03 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc0"),
                    "Path" => format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-litedb"),
                    "UnityVersion" => "2022.3.22f1",
                    "CreatedAt" => vrc_get_litedb::date!(2022-03-12 11:02:14),
                    "LastModified" => vrc_get_litedb::date!(2024-03-03 12:15:00),
                    "Type" => ProjectType::Avatars as i32,
                    "Favorite" => true,
                },
                vrc_get_litedb::document! {
                    "_id" => vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc1"),
                    "Path" => format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
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

#[tokio::test]
async fn sqlite_migrations_with_projects_in_database_projects_union() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    sqlite_migrations_with_projects_in_database_prepare(&env_dir, env_projects_str).await;
    std::fs::write(env_dir.join(VRC_GET_SETTINGS), r#"{")":"ProjectsUnion"}"#).unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let manage = ProjectManagement::start(io).await.unwrap(); // does migration

    // check data
    assert_eq!(
        load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
        defined_projects_in_settings_json!(
            env_projects_str,
            additional_projects = [
                format!("{env_projects_str}/litedb-good"),
                format!("{env_projects_str}/sqlite-only-no-id"),
                format!("{env_projects_str}/sqlite-only-with-id"),
                format!("{env_projects_str}/id-mismatch"),
                format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-sqlite"),
                format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
            ],
        ),
    );

    let get_project = |name: &str| {
        manage
            .find_project(&format!("{env_projects_str}/{name}"))
            .unwrap()
    };

    let id_mismatch = get_project("id-mismatch").unwrap();
    assert_eq!(
        id_mismatch.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"id-mismatch0"))
    );

    let sqlite_only_no_id = get_project("sqlite-only-no-id").unwrap();
    assert!(sqlite_only_no_id.litedb_objectid().is_none());

    let sqlite_only_with_id = get_project("sqlite-only-with-id").unwrap();
    assert!(sqlite_only_with_id.litedb_objectid().is_some());

    let litedb_good = get_project("litedb-good").unwrap();
    assert_eq!(
        litedb_good.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"litedbgoodpr"))
    );

    let patch_mismatch_0 = get_project("path-mismatch-0-sqlite-is-newer-sqlite").unwrap();
    assert_eq!(
        patch_mismatch_0.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc0"))
    );
    assert!(get_project("path-mismatch-0-sqlite-is-newer-litedb").is_none());

    let path_mismatch_1 = get_project("path-mismatch-1-litedb-is-newer-litedb").unwrap();
    assert_eq!(
        path_mismatch_1.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc1"))
    );
    assert!(get_project("path-mismatch-1-litedb-is-newer-sqlite").is_none());
}

#[tokio::test]
//#[cfg(false)]
async fn sqlite_migrations_with_projects_in_database_trust_litedb() {
    // initialize environment
    common::init_log();
    let env_dir = get_temp_path("environment");
    let env_projects = get_temp_path("env_projects");
    let env_projects_str = env_projects.to_str().unwrap();
    sqlite_migrations_with_projects_in_database_prepare(&env_dir, env_projects_str).await;
    std::fs::write(
        env_dir.join(VRC_GET_SETTINGS),
        r#"{"projectListSyncMode":"TrustLitedb"}"#,
    )
    .unwrap();

    // run code
    let io = &DefaultEnvironmentIo::new(env_dir.clone().into());
    let manage = ProjectManagement::start(io).await.unwrap(); // does migration

    // check data
    assert_eq!(
        load_projects_in_sqlite(env_dir.join(VRC_GET_SQLITE)),
        defined_projects_in_settings_json!(
            env_projects_str,
            additional_projects = [
                format!("{env_projects_str}/litedb-good"),
                // We keep sqlite-only-no-id since it does not have on ID and it means sqlite-only project,
                // VCC-incompatible projects
                format!("{env_projects_str}/sqlite-only-no-id"),
                format!("{env_projects_str}/id-mismatch"),
                format!("{env_projects_str}/path-mismatch-0-sqlite-is-newer-litedb"),
                format!("{env_projects_str}/path-mismatch-1-litedb-is-newer-litedb"),
            ],
        ),
    );

    let get_project = |name: &str| {
        manage
            .find_project(&format!("{env_projects_str}/{name}"))
            .unwrap()
    };

    let id_mismatch = get_project("id-mismatch").unwrap();
    assert_eq!(
        id_mismatch.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"id-mismatch0"))
    );

    let litedb_good = get_project("litedb-good").unwrap();
    assert_eq!(
        litedb_good.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"litedbgoodpr"))
    );

    let path_mismatch_0 = get_project("path-mismatch-0-sqlite-is-newer-litedb").unwrap();
    assert_eq!(
        path_mismatch_0.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc0"))
    );
    assert!(get_project("path-mismatch-0-sqlite-is-newer-sqlite").is_none());

    let path_mismatch_1 = get_project("path-mismatch-1-litedb-is-newer-litedb").unwrap();
    assert_eq!(
        path_mismatch_1.litedb_objectid(),
        Some(vrc_get_litedb::bson::ObjectId::from_bytes(*b"pathmismatc1"))
    );
    assert!(get_project("path-mismatch-1-litedb-is-newer-sqlite").is_none());
}
