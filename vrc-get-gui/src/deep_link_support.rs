use crate::commands::import_templates;
use arc_swap::ArcSwapOption;
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
#[allow(unused_imports)] // Manager is used only on linux
use tauri::{AppHandle, Emitter, Manager};
use url::{Host, Url};

static APP_HANDLE: ArcSwapOption<AppHandle> = ArcSwapOption::const_empty();

pub fn set_app_handle(handle: AppHandle) {
    APP_HANDLE.store(Some(Arc::new(handle)));
}

#[derive(Debug, Eq, PartialEq)]
enum DeepLink {
    AddRepository(AddRepositoryInfo),
}

fn parse_deep_link(deep_link: Url) -> Option<DeepLink> {
    if deep_link.scheme() != "vcc" {
        log::error!("Invalid deep link: {}", deep_link);
        return None;
    }

    if deep_link.host() != Some(Host::Domain("vpm")) {
        log::error!("Invalid deep link: {}", deep_link);
        return None;
    }

    match deep_link.path() {
        "/addRepo" => {
            // add repo
            let mut url = None;
            let mut headers = IndexMap::new();
            for (key, value) in deep_link.query_pairs() {
                match key.as_ref() {
                    "url" => {
                        if url.is_some() {
                            log::error!("Duplicate url query parameter");
                            return None;
                        }
                        let Some(parsed) = Url::parse(&value)
                            .ok()
                            .filter(|x| x.scheme() == "http" || x.scheme() == "https")
                        else {
                            log::error!("Invalid to remove url: {}", value);
                            return None;
                        };
                        url = Some(parsed);
                    }
                    "headers[]" => {
                        let (key, value) = value.split_once(':')?;
                        headers.insert(key.to_string(), value.to_string());
                    }
                    _ => {
                        log::error!("Unknown query parameter: {}", key);
                    }
                }
            }

            Some(DeepLink::AddRepository(AddRepositoryInfo {
                url: url?,
                headers,
            }))
        }
        _ => {
            log::error!("Unknown deep link: {}", deep_link);
            None
        }
    }
}

#[derive(specta::Type, serde::Serialize, Debug, Eq, PartialEq)]
pub struct AddRepositoryInfo {
    url: Url,
    headers: IndexMap<String, String>,
}

static PENDING_ADD_REPOSITORY: Mutex<Vec<AddRepositoryInfo>> = Mutex::new(Vec::new());

pub fn on_deep_link(deep_link: Url) {
    match parse_deep_link(deep_link) {
        None => {}
        Some(DeepLink::AddRepository(add_repository)) => {
            PENDING_ADD_REPOSITORY.lock().unwrap().push(add_repository);
            APP_HANDLE
                .load()
                .as_ref()
                .map(|handle| handle.emit("deep-link-add-repository", ()));
        }
    }
}

#[allow(unused_variables)]
pub fn should_install_deep_link(app: &AppHandle) -> bool {
    #[cfg(target_os = "linux")]
    if app.env().appimage.is_some() {
        return true;
    }

    cfg!(target_os = "windows")
}

static IMPORTED_NON_TOASTED_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn process_files(app: &AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let imported = import_templates(&app.state(), &files).await;
        app.emit("templates-imported", imported).ok();
        IMPORTED_NON_TOASTED_COUNT.fetch_add(1, Ordering::SeqCst);
    });
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_has_add_repository() -> bool {
    !PENDING_ADD_REPOSITORY.lock().unwrap().is_empty()
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_take_add_repository() -> Option<AddRepositoryInfo> {
    PENDING_ADD_REPOSITORY.lock().unwrap().pop()
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "macos")]
pub async fn deep_link_install_vcc(_app: AppHandle) {
    // for macos, nothing to do!
    log::error!("deep_link_install_vcc is not supported on macos");
}

#[tauri::command]
#[specta::specta]
#[cfg(windows)]
pub async fn deep_link_install_vcc(_app: AppHandle) {
    // for windows, install to registry
    fn impl_() -> std::io::Result<()> {
        let exe = std::env::current_exe()?;
        let exe = exe.to_string_lossy();

        let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .create_subkey("Software\\Classes\\vcc")?;
        key.set_value("URL Protocol", &"")?;
        let (default_icon, _) = key.create_subkey("DefaultIcon")?;
        default_icon.set_value("", &format!("\"{exe}\",0"))?;
        let (command, _) = key.create_subkey("shell\\open\\command")?;
        command.set_value("", &format!("\"{exe}\" link \"%1\""))?;
        Ok(())
    }

    if let Err(e) = impl_() {
        log::error!("Failed to install vcc deep link: {}", e);
    }
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "linux")]
pub async fn deep_link_install_vcc(app: AppHandle) {
    use tauri::Manager as _;
    // for linux, create a desktop entry
    // https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html

    let Some(home_dir) = dirs_next::data_dir() else {
        log::error!("Failed to get XDG_DATA_HOME");
        return;
    };
    let applications_dir = home_dir.join("applications");
    let desktop_file = applications_dir.join(format!(
        "{app_id}.desktop",
        app_id = "com.anatawa12.vrc_get"
    ));

    let Some(appimage_path) = app.env().appimage.and_then(|x| x.into_string().ok()) else {
        log::error!("Failed to get appimage path");
        return;
    };

    let contents = format!(
        r#"[Desktop Entry]
Type=Application
Name=ALCOM
Exec="{appimage_path}" link %u
NoDisplay=true
Terminal=false
MimeType=x-scheme-handler/vcc
Categories=Utility;
"#,
        appimage_path = escape(&appimage_path)
    );

    if let Err(e) = tokio::fs::create_dir_all(&applications_dir).await {
        log::error!("Failed to create applications directory: {}", e);
        return;
    }

    if let Err(e) = tokio::fs::write(&desktop_file, &contents).await {
        log::error!("Failed to write desktop file: {}", e);
        return;
    }

    log::info!("Desktop file created: {}", desktop_file.display());

    if let Err(e) = tokio::process::Command::new("update-desktop-database")
        .arg(applications_dir)
        .status()
        .await
    {
        log::error!("Failed to call update-desktop-database: {}", e);
    }

    fn escape(s: &str) -> String {
        s.replace('\\', r#"\\\\"#)
            .replace('`', r#"\\`"#)
            .replace('$', r#"\\$"#)
            .replace('"', r#"\\""#)
    }
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "macos")]
pub async fn deep_link_uninstall_vcc(_app: AppHandle) {
    // for macos, nothing to do!
    log::error!("deep_link_uninstall_vcc is not supported on macos");
}

#[tauri::command]
#[specta::specta]
#[cfg(windows)]
pub async fn deep_link_uninstall_vcc(_app: AppHandle) {
    // for windows, install to registry
    fn impl_() -> std::io::Result<()> {
        winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .delete_subkey_all("Software\\Classes\\vcc")?;
        Ok(())
    }

    if let Err(e) = impl_() {
        log::error!("Failed to install vcc deep link: {}", e);
    }
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "linux")]
pub async fn deep_link_uninstall_vcc(_app: AppHandle) {
    // for linux, create a desktop entry
    // https://specifications.freedesktop.org/desktop-entry-spec/desktop-entry-spec-latest.html

    let Some(home_dir) = dirs_next::data_dir() else {
        log::error!("Failed to get XDG_DATA_HOME");
        return;
    };
    let applications_dir = home_dir.join("applications");
    let desktop_file = applications_dir.join(format!(
        "{app_id}.desktop",
        app_id = "com.anatawa12.vrc_get"
    ));

    match tokio::fs::remove_file(&desktop_file).await {
        Ok(()) => {
            log::info!("Desktop file removed: {}", desktop_file.display());
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("Desktop file was not found: {}", desktop_file.display());
            return;
        }
        Err(e) => {
            log::error!("Failed to remove desktop file: {}", e);
            return;
        }
    }

    if let Err(e) = tokio::process::Command::new("update-desktop-database")
        .arg(applications_dir)
        .status()
        .await
    {
        log::error!("Failed to call update-desktop-database: {}", e);
    }
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_imported_clear_non_toasted_count() -> usize {
    IMPORTED_NON_TOASTED_COUNT.swap(0, Ordering::SeqCst)
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_reduce_imported_clear_non_toasted_count(reduce: usize) {
    IMPORTED_NON_TOASTED_COUNT.fetch_sub(reduce, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn parse_add_repo() {
        let deep_link =
            parse_deep_link(Url::parse("vcc://vpm/addRepo?url=https://example.com").unwrap())
                .unwrap();
        assert_eq!(
            deep_link,
            DeepLink::AddRepository(AddRepositoryInfo {
                url: Url::parse("https://example.com").unwrap(),
                headers: IndexMap::new(),
            })
        );

        let deep_link = parse_deep_link(
            Url::parse("vcc://vpm/addRepo?url=https%3A%2F%2Fvpm.anatawa12.com%2Fvpm.json").unwrap(),
        )
        .unwrap();
        assert_eq!(
            deep_link,
            DeepLink::AddRepository(AddRepositoryInfo {
                url: Url::parse("https://vpm.anatawa12.com/vpm.json").unwrap(),
                headers: IndexMap::new(),
            })
        );

        let deep_link = parse_deep_link(
            Url::parse("vcc://vpm/addRepo?url=https%3A%2F%2Fvpm.anatawa12.com%2Fvpm.json&headers[]=Authorization:test").unwrap()).unwrap();
        assert_eq!(
            deep_link,
            DeepLink::AddRepository(AddRepositoryInfo {
                url: Url::parse("https://vpm.anatawa12.com/vpm.json").unwrap(),
                headers: {
                    let mut map = IndexMap::new();
                    map.insert("Authorization".to_string(), "test".to_string());
                    map
                },
            })
        );
    }
}
