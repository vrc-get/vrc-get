use std::process::Command as ProcessCommand;
use std::sync::OnceLock;

use serde_json::Value as JsonValue;
type JsonObject = serde_json::Map<String, JsonValue>;

pub fn command() -> ProcessCommand {
    ProcessCommand::new("cargo")
}

pub fn cargo_metadata() -> &'static JsonObject {
    static CACHE: OnceLock<JsonObject> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut cmd = command();
        cmd.args(["metadata", "--format-version", "1"]);

        let output = cmd.output().expect("cargo metadata failed");
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            panic!("cargo metadata failed: {}", message);
        }

        std::mem::take(
            serde_json::from_slice::<JsonValue>(&output.stdout)
                .expect("cargo metadata failed: not a json")
                .as_object_mut()
                .expect("cargo metadata failed: not an object"),
        )
    })
}

pub fn workspace_root() -> &'static std::path::Path {
    cargo_metadata()
        .get("workspace_root")
        .and_then(JsonValue::as_str)
        .expect("workspace_root not found")
        .as_ref()
}

pub fn target_directory() -> &'static std::path::Path {
    cargo_metadata()
        .get("target_directory")
        .and_then(JsonValue::as_str)
        .expect("target_directory not found")
        .as_ref()
}

pub fn package_version(package_name: &str) -> &'static str {
    cargo_metadata()
        .get("packages")
        .and_then(JsonValue::as_array)
        .and_then(|packages| {
            packages
                .iter()
                .find(|p| p.get("name").and_then(JsonValue::as_str) == Some(package_name))
        })
        .and_then(|p| p.get("version").and_then(JsonValue::as_str))
        .unwrap_or_else(|| panic!("package_version not found for package: {package_name}"))
}

pub fn gui_version() -> &'static str {
    package_version("vrc-get-gui")
}
