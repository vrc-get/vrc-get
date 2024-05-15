use std::sync::Arc;

use crate::specta::IndexMapV2;
use arc_swap::ArcSwapOption;
use indexmap::IndexMap;
use tauri::{AppHandle, Manager};
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
                        url = Some(value.to_string());
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
                headers: IndexMapV2(headers),
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
    url: String,
    headers: IndexMapV2<String, String>,
}

static PENDING_ADD_REPOSITORY: ArcSwapOption<AddRepositoryInfo> = ArcSwapOption::const_empty();

pub fn on_deep_link(deep_link: Url) {
    match parse_deep_link(deep_link) {
        None => {}
        Some(DeepLink::AddRepository(add_repository)) => {
            PENDING_ADD_REPOSITORY.store(Some(Arc::new(add_repository)));
            APP_HANDLE
                .load()
                .as_ref()
                .map(|handle| handle.emit_all("deep-link-add-repository", ()));
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_has_add_repository() -> bool {
    PENDING_ADD_REPOSITORY.load().is_some()
}

#[tauri::command]
#[specta::specta]
pub fn deep_link_take_add_repository() -> Option<AddRepositoryInfo> {
    PENDING_ADD_REPOSITORY
        .swap(None)
        .map(|arc| Arc::try_unwrap(arc).ok().unwrap())
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "macos")]
pub async fn deep_link_install_vcc() {
    // for macos, nothing to do!
}

#[tauri::command]
#[specta::specta]
#[cfg(windows)]
// for windows, install to registry
pub async fn deep_link_install_vcc() {
    fn impl_() -> std::io::Result<()> {
        use winreg::reg_key;

        let exe = std::env::current_exe()?.to_string_lossy();

        let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .create_subkey("Software\\Classes\\vcc")?;
        key.set_value("URL Protocol", &"")?;
        let (default_icon, _) = key.create_subkey("DefaultIcon")?;
        default_icon.set_value("", format!("\"{exe}\",0"))?;
        let (command, _) = key.create_subkey("shell\\open\\command")?;
        command.set_value("", format!("\"{exe}\" link \"%1\""))?;
        Ok(())
    }

    if let Err(e) = impl_() {
        log::error!("Failed to install vcc deep link: {}", e);
    }
}

#[tauri::command]
#[specta::specta]
#[cfg(target_os = "linux")]
pub async fn deep_link_install_vcc() {
    log::error!("linux not supported yet")
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
                url: "https://example.com".to_string(),
                headers: IndexMapV2(IndexMap::new()),
            })
        );

        let deep_link = parse_deep_link(
            Url::parse("vcc://vpm/addRepo?url=https%3A%2F%2Fvpm.anatawa12.com%2Fvpm.json").unwrap(),
        )
        .unwrap();
        assert_eq!(
            deep_link,
            DeepLink::AddRepository(AddRepositoryInfo {
                url: "https://vpm.anatawa12.com/vpm.json".to_string(),
                headers: IndexMapV2(IndexMap::new()),
            })
        );

        let deep_link = parse_deep_link(
            Url::parse("vcc://vpm/addRepo?url=https%3A%2F%2Fvpm.anatawa12.com%2Fvpm.json&headers[]=Authorization:test").unwrap()).unwrap();
        assert_eq!(
            deep_link,
            DeepLink::AddRepository(AddRepositoryInfo {
                url: "https://vpm.anatawa12.com/vpm.json".to_string(),
                headers: IndexMapV2({
                    let mut map = IndexMap::new();
                    map.insert("Authorization".to_string(), "test".to_string());
                    map
                }),
            })
        );
    }
}
