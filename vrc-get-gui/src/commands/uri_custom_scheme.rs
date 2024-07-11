use serde::Serialize;
use tauri::http::{Request, Response, ResponseBuilder};
use tauri::{AppHandle, Manager};

use vrc_get_vpm::io::DefaultEnvironmentIo;

use crate::config::GuiConfigState;

pub fn handle_vrc_get_scheme(
    app: &AppHandle,
    request: &Request,
) -> Result<Response, Box<dyn std::error::Error>> {
    match request.uri() {
        "vrc-get:global-info.js" => global_info_json(app),
        _ => ResponseBuilder::new().status(404).body(b"bad".into()),
    }
}

pub fn global_info_json(app: &AppHandle) -> Result<Response, Box<dyn std::error::Error>> {
    let config = app.state::<GuiConfigState>();
    let io = app.state::<DefaultEnvironmentIo>();
    let config = config.load_sync(&io)?;

    // keep structure sync with global-info.ts
    #[derive(Serialize)]
    struct GlobalInfo<'a> {
        language: &'a str,
        theme: &'a str,
    }

    let global_info = GlobalInfo {
        language: &config.language,
        theme: &config.theme,
    };

    let mut script = b"globalThis.vrcGetGlobalInfo = ".to_vec();

    serde_json::to_writer(&mut script, &global_info)?;

    drop(config);

    ResponseBuilder::new()
        .status(200)
        .header("content-type", "application/javascript")
        .body(script)
}
