/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use serde::Serialize;
use std::borrow::Cow;
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager, UriSchemeContext, Wry};
use vrc_get_vpm::io::DefaultEnvironmentIo;

use crate::commands::DEFAULT_UNITY_ARGUMENTS;
use crate::state::GuiConfigState;

pub fn handle_vrc_get_scheme(
    app: UriSchemeContext<'_, Wry>,
    request: Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let url = request.uri();
    log::info!("recived request: {url}");
    if url.scheme().map(|x| x.as_str()) != Some("vrc-get") {
        return Response::builder()
            .status(404)
            .body(b"bad sceme".into())
            .unwrap();
    };
    match url.path() {
        "/global-info.js" => global_info_json(app.app_handle()),
        _ => Response::builder()
            .status(404)
            .body(b"bad url".into())
            .unwrap(),
    }
}

// keep structure sync with global-info.ts
#[derive(Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GlobalInfo<'a> {
    language: &'a str,
    theme: &'a str,
    version: Option<&'a str>,
    commit_hash: Option<&'a str>,
    os_type: &'a str,
    arch: &'a str,
    os_info: &'a str,
    webview_version: &'a str,
    app_data: &'a str,
    default_unity_arguments: &'a [&'a str],
    vpm_home_folder: &'a std::path::Path,
    check_for_updates: bool,
    setup_pages: Vec<(super::environment::config::SetupPages, &'static str)>,
    should_install_deep_link: bool,
}

pub fn global_info_json(app: &AppHandle) -> Response<Cow<'static, [u8]>> {
    let io = app.state::<DefaultEnvironmentIo>();
    let config = app.state::<GuiConfigState>();
    let config = config.get();

    let vpm_home_folder = io.inner().resolve("".as_ref());

    let os_type = cfg_select! {
        target_os = "macos" => "Darwin",
        target_os = "windows" => "WindowsNT",
        target_os = "linux" => "Linux",
    };

    let arch = cfg_select! {
        target_arch = "x86_64" => "x86_64",
        target_arch = "aarch64" => "aarch64",
    };

    let os_info = crate::os::os_info();

    let webview_version = tauri::webview_version();
    let webview_version = webview_version.as_deref().unwrap_or("unknown");

    // we do not support updater with rpm or deb
    let check_for_updates = cfg_select! {
        target_os = "macos" => true,
        target_os = "windows" => true,
        target_os = "linux" => app.env().appimage.is_some(),
    };

    let app_data = cfg_select! {
        windows => crate::os::app_data(),
        _ => "",
    };

    let should_install_deep_link = crate::deep_link_support::should_install_deep_link(app);

    let global_info = GlobalInfo {
        language: &config.language,
        theme: &config.theme,
        version: Some(env!("CARGO_PKG_VERSION")),
        commit_hash: option_env!("COMMIT_HASH"),
        os_type,
        arch,
        os_info,
        webview_version,
        app_data,
        default_unity_arguments: DEFAULT_UNITY_ARGUMENTS,
        vpm_home_folder: &vpm_home_folder,
        check_for_updates,
        setup_pages: super::environment::config::SetupPages::pages(app)
            .iter()
            .map(|&page_id| (page_id, page_id.path()))
            .collect(),
        should_install_deep_link,
    };

    let mut script = b"globalThis.vrcGetGlobalInfo = ".to_vec();

    serde_json::to_writer(&mut script, &global_info).expect("failed to serialize global info");

    drop(config);

    Response::builder()
        .status(200)
        .header("content-type", "application/javascript")
        .body(script.into())
        .unwrap()
}
