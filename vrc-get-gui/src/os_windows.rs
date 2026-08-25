//! OS-specific functionality.

//! This module is for creating `cmd.exe /d /c start "Name"
//! "path/to/executable" args` command correctly.
//!
//! Since the `cmd.exe` has a unique escape sequence behavior,
//! It's necessary to escape the path and arguments correctly.
//!
//! I wrote this module based on [BatBadBut] article.
//!
//! [BatBadBut]: https://flatt.tech/research/posts/batbadbut-you-cant-securely-execute-commands-on-windows/#as-a-developer

use std::ffi::{OsStr, OsString};
use std::fs::OpenOptions;
use std::io;
use std::mem::MaybeUninit;
use std::os::windows::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use sysinfo::System;
use tokio::process::Command;
use windows::Win32::Foundation::{
    ERROR_LOCK_VIOLATION, ERROR_SHARING_VIOLATION, HANDLE, HWND, LPARAM,
};
use windows::Win32::Storage::FileSystem::{
    LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx, UnlockFileEx,
};
use windows::Win32::System::IO::OVERLAPPED;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FLASHW_TIMERNOFG, FLASHW_TRAY, FLASHWINFO, FlashWindowEx, GetClassNameW, GetMenu,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow,
    ShowWindow,
};
use windows::core::BOOL;

use super::BringUnityToFrontResult;
use crate::unity_process::{UnityProcess, paths_match};

const CREATE_NO_WINDOW: u32 = 0x08000000;
const LOCK_RANGE_LOW: u32 = u32::MAX;
const LOCK_RANGE_HIGH: u32 = u32::MAX;
const UNITY_EDITOR_WINDOW_CLASS: &str = "UnityContainerWndClass";
const UNITY_RUNTIME_CACHE_TTL: Duration = Duration::from_secs(1);
pub(crate) const CAN_BRING_UNITY_TO_FRONT: bool = true;
pub(crate) const CAN_DETECT_UNITY_EDITOR_READY: bool = true;

#[derive(Clone, Copy)]
struct UnityEditorWindowHandle(HWND);

// SAFETY: HWND is an opaque window identifier that Windows APIs allow using from
// other threads. Unity owns the window, and the handle is validated before use.
unsafe impl Send for UnityEditorWindowHandle {}

struct UnityEditorWindow {
    project_path: PathBuf,
    process_id: u32,
    window: UnityEditorWindowHandle,
}

pub(crate) struct UnityRuntimeCache {
    system: System,
    refreshed_at: Option<Instant>,
    editor_windows: Vec<UnityEditorWindow>,
}

pub(crate) async fn start_command(
    name: &OsStr,
    path: &OsStr,
    args: &[&OsStr],
) -> std::io::Result<()> {
    // prepare
    let mut cmd_args = Vec::new();
    cmd_args.extend("/E:ON /V:OFF /d /c start /b ".encode_utf16());
    append_cmd_escaped(&mut cmd_args, name.encode_wide());
    cmd_args.push(b' ' as u16);

    append_cmd_escaped(&mut cmd_args, path.encode_wide());

    for arg in args {
        cmd_args.push(b' ' as u16);
        append_cmd_escaped(&mut cmd_args, arg.encode_wide());
    }

    // execute
    let status = Command::new("cmd.exe")
        .raw_arg(OsString::from_wide(&cmd_args))
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .await?;

    if !status.success() {
        Err(std::io::Error::other(format!(
            "cmd.exe /E:ON /V:OFF /d /c start /d failed with status: {status}",
        )))
    } else {
        Ok(())
    }
}

// %%cd:~,%
const PERCENT_ESCAPED: &[u16] = &[0x25, 0x25, 0x63, 0x64, 0x3a, 0x7e, 0x2c, 0x25];

// based on https://flatt.tech/research/posts/batbadbut-you-cant-securely-execute-commands-on-windows/#as-a-developer
fn append_cmd_escaped(args: &mut Vec<u16>, arg: impl Iterator<Item = u16>) {
    // Enclose the argument with double quotes (").
    args.push('"' as u16);

    let mut backslash = 0;
    for x in arg {
        if x == b'%' as u16 {
            args.extend_from_slice(PERCENT_ESCAPED);
        } else if x == b'"' as u16 {
            // Replace the backslash (\) in front of the double quote (") with two backslashes (\\).
            //  To implement that, append the backslashes again
            args.extend(std::iter::repeat(b'\\' as u16).take(backslash));
            // Replace the double quote (") with two double quotes ("").
            args.push(b'"' as u16);
            args.push(b'"' as u16);
        } else if x == '\n' as u16 {
            // Remove newline characters (\n).
        } else {
            args.push(x);
        }

        // count b'\\'
        if x == b'\\' as u16 {
            backslash += 1;
        } else {
            backslash = 0;
        }
    }

    // Enclose the argument with double quotes (").
    args.push('"' as u16);
}

pub(crate) fn is_locked(path: &Path) -> io::Result<bool> {
    let file = match OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(error) if error.raw_os_error() == Some(ERROR_SHARING_VIOLATION.0 as i32) => {
            return Ok(true);
        }
        Err(error) => return Err(error),
    };

    unsafe {
        let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
        overlapped.Anonymous.Anonymous.Offset = 0;
        overlapped.Anonymous.Anonymous.OffsetHigh = 0;
        match LockFileEx(
            HANDLE(file.as_raw_handle()),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            None,
            LOCK_RANGE_LOW,
            LOCK_RANGE_HIGH,
            &mut overlapped,
        ) {
            Err(ref e) if e.code() == ERROR_LOCK_VIOLATION.into() => {
                // ERROR_LOCK_VIOLATION means it's already locked
                return Ok(true);
            }
            // other error
            Err(e) => return Err(e.into()),
            Ok(()) => {}
        }
        // Locking succeeded, so no other process owns the range.
        let mut overlapped: OVERLAPPED = MaybeUninit::zeroed().assume_init();
        overlapped.Anonymous.Anonymous.Offset = 0;
        overlapped.Anonymous.Anonymous.OffsetHigh = 0;
        UnlockFileEx(
            HANDLE(file.as_raw_handle()),
            None,
            LOCK_RANGE_LOW,
            LOCK_RANGE_HIGH,
            &mut overlapped,
        )?;
        Ok(false)
    }
}

impl UnityRuntimeCache {
    pub(crate) fn new() -> Self {
        Self {
            system: System::new(),
            refreshed_at: None,
            editor_windows: Vec::new(),
        }
    }

    pub(crate) fn is_editor_ready(&mut self, project_path: &Path) -> bool {
        if let Err(error) = self.refresh_if_needed(Instant::now()) {
            log::debug!("Checking which Unity editor windows are ready: {error}");
        }

        self.find_editor_window(project_path).is_some()
    }

    pub(crate) fn bring_unity_to_front(
        &mut self,
        project_path: &Path,
    ) -> io::Result<BringUnityToFrontResult> {
        self.refresh_if_needed(Instant::now())?;

        let mut editor_window = self.find_editor_window(project_path);
        if editor_window.is_none_or(|(process_id, window)| {
            !is_cached_unity_editor_window_valid(process_id, window)
        }) {
            self.refresh(Instant::now())?;
            editor_window = self.find_editor_window(project_path);
        }

        let Some((process_id, window)) = editor_window else {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        };
        if !is_cached_unity_editor_window_valid(process_id, window) {
            return Ok(BringUnityToFrontResult::WindowNotFound);
        }

        activate_unity_editor_window(window.0)
    }

    pub(crate) fn invalidate(&mut self) {
        self.refreshed_at = None;
        self.editor_windows.clear();
    }

    fn find_editor_window(&self, project_path: &Path) -> Option<(u32, UnityEditorWindowHandle)> {
        self.editor_windows
            .iter()
            .find(|editor_window| paths_match(&editor_window.project_path, project_path))
            .map(|editor_window| (editor_window.process_id, editor_window.window))
    }

    fn refresh_if_needed(&mut self, now: Instant) -> io::Result<()> {
        if self.should_refresh(now) {
            self.refresh(now)?;
        }
        Ok(())
    }

    fn should_refresh(&self, now: Instant) -> bool {
        self.refreshed_at.is_none_or(|refreshed_at| {
            now.saturating_duration_since(refreshed_at) >= UNITY_RUNTIME_CACHE_TTL
        })
    }

    fn refresh(&mut self, now: Instant) -> io::Result<()> {
        let processes = crate::unity_process::refresh_unity_processes(&mut self.system);
        let editor_windows = find_unity_editor_windows(processes);
        self.refreshed_at = Some(now);

        match editor_windows {
            Ok(editor_windows) => {
                self.editor_windows = editor_windows;
                Ok(())
            }
            Err(error) => {
                self.editor_windows.clear();
                Err(error)
            }
        }
    }
}

fn activate_unity_editor_window(window: HWND) -> io::Result<BringUnityToFrontResult> {
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }

        if SetForegroundWindow(window).as_bool() {
            return Ok(BringUnityToFrontResult::BroughtToFront);
        }

        let flash_info = FLASHWINFO {
            cbSize: size_of::<FLASHWINFO>() as u32,
            hwnd: window,
            dwFlags: FLASHW_TRAY | FLASHW_TIMERNOFG,
            uCount: 3,
            dwTimeout: 0,
        };
        // The return value describes whether the window was active before this call,
        // not whether the attention request succeeded.
        let _ = FlashWindowEx(&flash_info);
    }

    Ok(BringUnityToFrontResult::FailedToBringToFront)
}

fn is_cached_unity_editor_window_valid(process_id: u32, window: UnityEditorWindowHandle) -> bool {
    let window = window.0;
    let mut current_process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut current_process_id));
    }

    current_process_id == process_id && unsafe { is_unity_editor_window(window) }
}

fn find_unity_editor_windows(processes: Vec<UnityProcess>) -> io::Result<Vec<UnityEditorWindow>> {
    if processes.is_empty() {
        return Ok(Vec::new());
    }

    let mut context = FindUnityWindowsContext {
        processes,
        editor_windows: Vec::new(),
    };
    let context_pointer = &mut context as *mut FindUnityWindowsContext;

    unsafe {
        EnumWindows(
            Some(find_unity_windows),
            LPARAM(context_pointer.expose_provenance() as isize),
        )?;
    }

    Ok(context.editor_windows)
}

struct FindUnityWindowsContext {
    processes: Vec<UnityProcess>,
    editor_windows: Vec<UnityEditorWindow>,
}

unsafe extern "system" fn find_unity_windows(window: HWND, parameter: LPARAM) -> BOOL {
    let context_pointer =
        std::ptr::with_exposed_provenance_mut::<FindUnityWindowsContext>(parameter.0 as usize);
    let context = unsafe { &mut *context_pointer };

    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, Some(&mut process_id));
    }
    let Some(process) = context
        .processes
        .iter()
        .find(|process| process.process_id == process_id)
    else {
        return BOOL(1);
    };

    if unsafe { is_unity_editor_window(window) }
        && !context
            .editor_windows
            .iter()
            .any(|editor_window| paths_match(&editor_window.project_path, &process.project_path))
    {
        context.editor_windows.push(UnityEditorWindow {
            project_path: process.project_path.clone(),
            process_id,
            window: UnityEditorWindowHandle(window),
        });
    }

    BOOL(1)
}

unsafe fn is_unity_editor_window(window: HWND) -> bool {
    let visible = unsafe { IsWindowVisible(window).as_bool() };
    let has_menu = !unsafe { GetMenu(window) }.0.is_null();
    if !visible || !has_menu {
        return false;
    }

    let mut class_name = [0u16; 64];
    let class_name_length = unsafe { GetClassNameW(window, &mut class_name) };
    if class_name_length <= 0 {
        return false;
    }

    let class_name = String::from_utf16_lossy(&class_name[..class_name_length as usize]);
    is_unity_editor_window_metadata(&class_name, visible, has_menu)
}

fn is_unity_editor_window_metadata(class_name: &str, visible: bool, has_menu: bool) -> bool {
    visible && has_menu && class_name == UNITY_EDITOR_WINDOW_CLASS
}

pub fn os_info() -> &'static str {
    static OS_INFO: OnceLock<String> = OnceLock::new();

    fn compute_os_info() -> String {
        if let Ok(full_info) = try_get_wmi_info() {
            return full_info;
        }

        get_basic_version()
    }

    fn try_get_wmi_info() -> Result<String, ()> {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;
        use wmi::WMIConnection;

        let (sender, receiver) = mpsc::channel::<Result<String, ()>>();

        thread::spawn(move || {
            use serde::Deserialize;

            #[allow(non_camel_case_types)]
            #[allow(non_snake_case)]
            #[derive(Deserialize, Debug)]
            struct Win32_OperatingSystem {
                #[serde(rename = "Caption")]
                caption: String,
                #[serde(rename = "Version")]
                version: String,
            }

            let wmi_con = match WMIConnection::new() {
                Ok(con) => con,
                Err(_) => {
                    let _ = sender.send(Err(()));
                    return;
                }
            };

            match wmi_con.query::<Win32_OperatingSystem>() {
                Ok(mut results) => {
                    if let Some(os) = results.pop() {
                        let _ = sender.send(Ok(format!("{} ({})", os.caption, os.version)));
                    } else {
                        let _ = sender.send(Err(()));
                    }
                }
                Err(_) => {
                    let _ = sender.send(Err(()));
                }
            }
        });

        match receiver.recv_timeout(Duration::from_secs(2)) {
            Ok(Ok(info)) => Ok(info),
            Ok(Err(_)) | Err(_) => Err(()),
        }
    }

    fn get_basic_version() -> String {
        use windows::Wdk::System::SystemServices::RtlGetVersion;
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;

        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };

        unsafe {
            if RtlGetVersion(&mut info).is_err() {
                return "Unknown".to_string();
            }
        }

        let ex_version = &info.szCSDVersion[..];
        let ex_version = &ex_version[..ex_version
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(ex_version.len())];
        let ex_version = String::from_utf16_lossy(ex_version);
        let ex_version = if ex_version.is_empty() {
            "".to_string()
        } else {
            format!(" ({ex_version})")
        };

        format!(
            "Windows {}.{}.{}{}",
            info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber, ex_version,
        )
    }

    OS_INFO.get_or_init(compute_os_info)
}

pub fn local_app_data() -> &'static str {
    static LOCAL_APP_DATA: OnceLock<String> = OnceLock::new();

    LOCAL_APP_DATA.get_or_init(|| {
        dirs_next::cache_dir()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default()
    })
}

pub fn app_data() -> &'static str {
    static APP_DATA: OnceLock<String> = OnceLock::new();

    APP_DATA.get_or_init(|| {
        // AppData is the parent directory of LocalAppData (AppData\Local)
        std::path::Path::new(local_app_data())
            .parent()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default()
    })
}

pub use open::that as open_that;

pub fn initialize(_: tauri::AppHandle) {
    // nothing to initialize
}

pub fn is_noexec(_path: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_an_unlocked_file_as_unlocked() {
        let file = tempfile::NamedTempFile::new().unwrap();

        assert!(!is_locked(file.path()).unwrap());
    }

    #[test]
    fn reports_a_sharing_violation_as_locked() {
        let temporary_path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
        let _locked_file = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&temporary_path)
            .unwrap();

        assert!(is_locked(&temporary_path).unwrap());
    }

    #[test]
    fn reports_a_competing_byte_range_lock_as_locked() {
        let temporary_file = tempfile::NamedTempFile::new().unwrap();
        let locked_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(temporary_file.path())
            .unwrap();

        let mut overlapped: OVERLAPPED = unsafe { MaybeUninit::zeroed().assume_init() };
        unsafe {
            LockFileEx(
                HANDLE(locked_file.as_raw_handle()),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                None,
                LOCK_RANGE_LOW,
                LOCK_RANGE_HIGH,
                &mut overlapped,
            )
            .unwrap();
        }

        let detected = is_locked(temporary_file.path()).unwrap();

        let mut overlapped: OVERLAPPED = unsafe { MaybeUninit::zeroed().assume_init() };
        unsafe {
            UnlockFileEx(
                HANDLE(locked_file.as_raw_handle()),
                None,
                LOCK_RANGE_LOW,
                LOCK_RANGE_HIGH,
                &mut overlapped,
            )
            .unwrap();
        }

        assert!(detected);
    }

    #[test]
    fn recognizes_a_visible_unity_editor_window_with_a_menu() {
        assert!(is_unity_editor_window_metadata(
            UNITY_EDITOR_WINDOW_CLASS,
            true,
            true
        ));
    }

    #[test]
    fn rejects_unity_startup_and_utility_windows() {
        assert!(!is_unity_editor_window_metadata(
            "UnitySplashWndClass",
            true,
            true
        ));
        assert!(!is_unity_editor_window_metadata(
            UNITY_EDITOR_WINDOW_CLASS,
            true,
            false
        ));
        assert!(!is_unity_editor_window_metadata(
            UNITY_EDITOR_WINDOW_CLASS,
            false,
            true
        ));
    }

    #[test]
    fn runtime_cache_expires_after_its_ttl() {
        let mut cache = UnityRuntimeCache::new();
        let refreshed_at = Instant::now();
        cache.refreshed_at = Some(refreshed_at);

        assert!(
            !cache
                .should_refresh(refreshed_at + UNITY_RUNTIME_CACHE_TTL - Duration::from_millis(1))
        );
        assert!(cache.should_refresh(refreshed_at + UNITY_RUNTIME_CACHE_TTL));
    }

    #[test]
    fn invalidating_runtime_cache_clears_cached_windows() {
        let mut cache = UnityRuntimeCache::new();
        cache.refreshed_at = Some(Instant::now());
        cache.editor_windows.push(UnityEditorWindow {
            project_path: PathBuf::from("project"),
            process_id: 1,
            window: UnityEditorWindowHandle(HWND::default()),
        });

        cache.invalidate();

        assert!(cache.refreshed_at.is_none());
        assert!(cache.editor_windows.is_empty());
    }
}
