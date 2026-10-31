use crate::io;
use crate::version::UnityVersion;
use serde::Deserialize;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;
use std::str::from_utf8;
use tokio::process::Command;

// avoids flashing a console window since unity.exe is console-subsystem
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Deserialize)]
struct EditorsResponse {
    success: bool,
    #[serde(default)]
    data: Vec<Editor>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Deserialize)]
struct Editor {
    version: String,
    location: String,
}

async fn call_unity_cli(unity_cli_path: &OsStr, args: &[&OsStr]) -> io::Result<Output> {
    let mut command = Command::new(unity_cli_path);
    command
        .args(args)
        .env("UNITY_NON_INTERACTIVE", "1")
        .env("UNITY_NO_BANNER", "1")
        .env("UNITY_NO_UPDATE_CHECK", "1")
        .env("UNITY_NO_CRASH_REPORT", "1");
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command.output().await
}

pub async fn load_unity_by_calling_unity_cli(
    unity_cli_path: &OsStr,
) -> io::Result<Vec<(UnityVersion, PathBuf)>> {
    let output = call_unity_cli(
        unity_cli_path,
        &["editors".as_ref(), "-i".as_ref(), "--json".as_ref()],
    )
    .await?;

    if !output.status.success() {
        // invalid usage exits without writing the json envelope
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unity CLI failed to get installed unity versions: {}",
                from_utf8(&output.stderr).unwrap_or_default().trim()
            ),
        ));
    }

    Ok(map_editors(parse_editors(&output.stdout)?))
}

fn parse_editors(stdout: &[u8]) -> io::Result<Vec<Editor>> {
    let response: EditorsResponse = serde_json::from_slice(stdout)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid json from unity cli"))?;

    if !response.success {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Unity CLI failed to get installed unity versions: {}",
                response.errors.join(", ")
            ),
        ));
    }

    Ok(response.data)
}

fn map_editors(editors: Vec<Editor>) -> Vec<(UnityVersion, PathBuf)> {
    let mut result = Vec::new();

    for editor in editors {
        let Some(version) = UnityVersion::parse(&editor.version) else {
            continue;
        };

        result.push((version, PathBuf::from(editor.location)));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(stdout: &str) -> io::Result<Vec<(UnityVersion, PathBuf)>> {
        parse_editors(stdout.as_bytes()).map(map_editors)
    }

    #[test]
    fn load_editors() {
        let editors = load(
            r#"{
              "success": true,
              "command": "editors",
              "data": [
                {
                  "version": "2022.3.22f1",
                  "alias": "",
                  "architecture": "x86_64",
                  "location": "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.22f1\\Editor\\Unity.exe",
                  "modules": "Android, Android SDK & NDK Tools, iOS, OpenJDK",
                  "default": false
                }
              ],
              "errors": [],
              "warnings": []
            }"#,
        )
        .unwrap();

        assert_eq!(
            editors,
            vec![(
                UnityVersion::parse("2022.3.22f1").unwrap(),
                PathBuf::from(
                    "C:\\Program Files\\Unity\\Hub\\Editor\\2022.3.22f1\\Editor\\Unity.exe"
                )
            )]
        );
    }

    #[test]
    fn prefer_version_over_alias() {
        let editors = load(
            r#"{"success":true,"data":[{"version":"6000.0.58f1","alias":"6.0.58f1",
              "location":"/Applications/Unity/Hub/Editor/6000.0.58f1/Unity.app"}],"errors":[]}"#,
        )
        .unwrap();

        assert_eq!(
            editors,
            vec![(
                UnityVersion::parse("6000.0.58f1").unwrap(),
                PathBuf::from("/Applications/Unity/Hub/Editor/6000.0.58f1/Unity.app")
            )]
        );
    }

    #[test]
    fn skip_editors_with_invalid_version() {
        let editors = load(
            r#"{"success":true,"data":[
              {"version":"not-a-version","location":"/nonexistent"},
              {"version":"2019.4.31f1","location":"/Applications/Unity/Hub/Editor/2019.4.31f1/Unity.app"}
            ],"errors":[]}"#,
        )
        .unwrap();

        assert_eq!(
            editors,
            vec![(
                UnityVersion::parse("2019.4.31f1").unwrap(),
                PathBuf::from("/Applications/Unity/Hub/Editor/2019.4.31f1/Unity.app")
            )]
        );
    }

    #[test]
    fn ignore_unknown_fields() {
        let editors = load(
            r#"{"success":true,"unknown":0,"data":[
              {"version":"2022.3.22f1","location":"/Applications/Unity/Hub/Editor/2022.3.22f1/Unity.app","unknown":0}
            ],"errors":[]}"#,
        )
        .unwrap();

        assert_eq!(editors.len(), 1);
    }

    #[test]
    fn error_for_unsuccessful_response() {
        assert!(load(r#"{"success":false,"data":[],"errors":["failed"]}"#).is_err());
    }

    #[test]
    fn error_for_non_json_output() {
        assert!(load("error: unexpected argument 'editor' found").is_err());
    }
}
