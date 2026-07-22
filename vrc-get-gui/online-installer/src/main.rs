#![windows_subsystem = "windows"]

use crate::common::{RemoteRelease, VerifySignatureError, get_updater_url};
use crate::is_wow64_process_2::HostArch;
use std::io::Write;
use std::os::windows::prelude::*;
use std::process::exit;
use std::sync::{Arc, Mutex};
use winsafe::{self as w, co, gui, prelude::*};

#[path = "../../src/updater/common.rs"]
#[allow(dead_code)]
mod common;

const WM_DOWNLOAD_DONE: co::WM = unsafe { co::WM::from_raw(co::WM::APP.raw() + 1) };
const TIMER_ID: usize = 1;
const TIMER_MS: u32 = 1000;

#[derive(Clone)]
struct WaitWnd {
    wnd: gui::WindowMain,
    hwnd: Arc<Mutex<winsafe::HWND>>,
    result_space: Arc<Mutex<Option<Result<tempfile::TempPath, String>>>>,
}

impl WaitWnd {
    fn new() -> Self {
        let wnd = gui::WindowMain::new(gui::WindowMainOpts {
            title: "Setup",
            size: (300, 90),
            style: gui::WindowMainOpts::default().style
                & !co::WS::VISIBLE      // start hidden
                & !co::WS::MAXIMIZEBOX
                & !co::WS::MINIMIZEBOX,
            ..Default::default()
        });

        let _ = gui::Label::new(
            &wnd,
            gui::LabelOpts {
                text: "Downloading installer, please wait...",
                position: (12, 30),
                size: (276, 20),
                ..Default::default()
            },
        );

        Self {
            wnd,
            hwnd: Arc::new(Mutex::new(winsafe::HWND::NULL)),
            result_space: Arc::new(Mutex::new(None)),
        }
    }

    fn events(&self) {
        let self2 = self.clone();
        self.wnd.on().wm_create(move |_| {
            eprintln!("wm_create");
            self2.wnd.hwnd().SetTimer(TIMER_ID, TIMER_MS, None)?;
            *self2.hwnd.lock().unwrap() = unsafe { self2.wnd.hwnd().raw_copy() };
            Ok(0)
        });

        let self3 = self.clone();
        self.wnd.on().wm_timer(TIMER_ID, move || {
            eprintln!("timer: TIMER_ID");
            self3.wnd.hwnd().KillTimer(TIMER_ID).ok();
            self3.wnd.hwnd().ShowWindow(co::SW::SHOW);
            Ok(())
        });

        let self4 = self.clone();
        self.wnd.on().wm(WM_DOWNLOAD_DONE, move |_| {
            eprintln!("WM_DOWNLOAD_DONE");
            self4.wnd.hwnd().KillTimer(TIMER_ID).ok(); // no-op if already fired/killed

            if let Err(message) = self4.result_space.lock().unwrap().as_ref().unwrap() {
                display_error_and_exit(
                    Some(self4.wnd.hwnd()),
                    &format!("Error downloading ALCOM offline installer: {message}"),
                );
            }

            self4.wnd.hwnd().DestroyWindow()?; // triggers WM_DESTROY -> PostQuitMessage
            Ok(0)
        });
    }

    fn run(&self, use_stable: bool) -> w::AnyResult<i32> {
        let hwnd = self.hwnd.clone();
        let result_slot = self.result_space.clone();

        std::thread::spawn(move || {
            let result = download_installer(use_stable); // your winhttp call, no progress needed
            eprintln!("Downloading updater finished with result: {result:?}");

            result_slot.lock().unwrap().replace(result);
            let hwnd = unsafe { hwnd.lock().unwrap().raw_copy() };
            let post_message =
                unsafe { hwnd.PostMessage(winsafe::msg::Wm::new(WM_DOWNLOAD_DONE, 0, 0)) };
            eprintln!("sent WM_DOWNLOAD_DONE: {post_message:?}");
        });

        self.wnd.run_main(Some(co::SW::HIDE))
    }
}

fn main() {
    let mut args = std::env::args_os().peekable();
    let _ = args.next(); // exe name
    let mut use_stable = true;
    if let Some(arg) = args.peek()
        && let Some(arg) = arg.as_os_str().to_str()
    {
        if matches!(arg, "/BETA" | "/beta" | "-BETA" | "-beta" | "--beta") {
            args.next(); // use
            use_stable = false;
        } else if matches!(
            arg,
            "/ONLINE-INSTALLER-HELP"
                | "/online-installer-help"
                | "-ONLINE-INSTALLER-HELP"
                | "-online-installer-help"
                | "--online-installer-help"
        ) {
            args.next(); // use
            eprintln!("The ALCOM online installer");
            eprintln!(
                "Downloads installer from updater endpoint, validate the installer, and installs the ALCOM."
            );
            eprintln!(
                "alcom-online-installer.exe [ONLINE-INSTALLER-OPTIONS] {{INSTALLER-OPTIONS}}"
            );
            eprintln!("ONLINE-INSTALLER-OPTIONS");
            eprintln!("\t/ONLINE-INSTALLER-HELP /online-installer-help");
            eprintln!("\t-ONLINE-INSTALLER-HELP -online-installer-help");
            eprintln!("\t--online-installer-help");
            eprintln!("\t\tShow this help message and exit.");
            eprintln!("\t/BETA /beta -BETA -beta --beta");
            eprintln!(
                "\t\tDownload and execute beta version of installer instead of stable version."
            );
            exit(0);
        }
    }

    let wnd = WaitWnd::new();
    wnd.events();
    wnd.run(use_stable).ok();

    let Ok(path) = wnd.result_space.lock().unwrap().take().unwrap() else {
        return;
    };

    let mut exit_code: u32 = 0;

    unsafe {
        let path = windows_strings::HSTRING::from_wide(
            &commandline_param_builder::build_command_line_passthrough(&path),
        );
        let mut process_info =
            windows_sys::Win32::System::Threading::PROCESS_INFORMATION::default();
        let startup_info = windows_sys::Win32::System::Threading::STARTUPINFOW::default();

        eprintln!("starting instlaler with command line: '{path}'",);

        let ok = windows_sys::Win32::System::Threading::CreateProcessW(
            windows_sys::core::PCWSTR::default(),
            path.as_ptr() as *mut _,
            std::ptr::null(),
            std::ptr::null(),
            false.into(),
            0,
            std::ptr::null(),
            std::ptr::null(),
            &startup_info,
            &mut process_info,
        );

        if ok == 0 {
            let last_error = std::io::Error::last_os_error();
            display_error_and_exit(
                None,
                &format!(
                    "Error launching ALCOM offline installer: {message}",
                    message = last_error.to_string()
                ),
            );
        }

        let wait_result = windows_sys::Win32::System::Threading::WaitForSingleObject(
            process_info.hProcess,
            windows_sys::Win32::System::Threading::INFINITE,
        );
        if wait_result == windows_sys::Win32::Foundation::WAIT_FAILED {
            let last_error = std::io::Error::last_os_error();
            windows_sys::Win32::Foundation::CloseHandle(process_info.hProcess);
            windows_sys::Win32::Foundation::CloseHandle(process_info.hThread);
            display_error_and_exit(
                None,
                &format!(
                    "Error launching ALCOM offline installer: {message}",
                    message = last_error.to_string()
                ),
            );
        }

        windows_sys::Win32::System::Threading::GetExitCodeProcess(
            process_info.hProcess,
            &mut exit_code,
        );

        windows_sys::Win32::Foundation::CloseHandle(process_info.hProcess);
        windows_sys::Win32::Foundation::CloseHandle(process_info.hThread);
    }

    drop(path);

    exit(exit_code.cast_signed());
}

fn download_installer(use_stable: bool) -> Result<tempfile::TempPath, String> {
    let client = winhttp::Client::builder()
        .user_agent(concat!("AlcomOnlineInstaller/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap();
    let url = get_updater_url(use_stable);
    eprintln!("Downloading updater json from {url}");
    let response = client
        .request("GET", &url)
        .header("Accept", "application/json")
        .send()
        .map_err(|e| e.to_string())?;

    // 2xx but not 204 since 204 does not include body.
    if response.status < 200 || response.status >= 300 || response.status == 204 {
        eprintln!(
            "Error downloading ALCOM offline installer: Server returns nonsuccessful status code: {status}",
            status = response.status,
        );
        eprintln!("Response body:");
        eprintln!("{}", String::from_utf8_lossy(&response.body));
        return Err(format!(
            "Server returns nonsuccessful status code: {status}",
            status = response.status
        ));
    }
    eprintln!("Downloading updater json completes");

    let Ok(json_string) = str::from_utf8(&response.body) else {
        eprintln!("Error downloading ALCOM offline installer: updater.json is NOT URF-8");
        eprintln!("Response body lossy:");
        eprintln!("{}", String::from_utf8_lossy(&response.body));
        return Err("updater.json is NOT UTF-8".into());
    };

    let release = serde_json::from_str::<RemoteRelease>(&json_string)
        .map_err(|e| format!("Failed to deserialize RemoteRelease: {e}"))?;

    let platforms = match is_wow64_process_2::get_host_arch() {
        HostArch::X64 => &["windows-x86_64"][..],
        HostArch::Arm64 => &["windows-aarch64", "windows-x86_64"],
        _ => &[],
    };

    let Some(platform) = platforms
        .iter()
        .filter_map(|platform| release.platforms.get(*platform))
        .next()
    else {
        return Err("No supported platform is available".into());
    };

    eprintln!("Downloading offline installer from {}", platform.url);
    let response = client
        .request("GET", &platform.url.to_string())
        .header("Accept", "application/octet-stream")
        .send()
        .map_err(|e| e.to_string())?;

    // 2xx but not 204 since 204 does not include body.
    if response.status < 200 || response.status >= 300 || response.status == 204 {
        eprintln!(
            "Error downloading ALCOM offline installer: Server returns nonsuccessful status code: {status}",
            status = response.status,
        );
        eprintln!("Response body:");
        eprintln!("{}", String::from_utf8_lossy(&response.body));
        return Err(format!(
            "Server returns nonsuccessful status code: {status}",
            status = response.status
        ));
    }

    match common::verify_signature(&response.body, &platform.signature, common::PUBLIC_KEY) {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("Verification failed");
            return Err("Verification failed".to_string());
        }
        Err(e) => match e {
            VerifySignatureError::InvalidBase64(e) => {
                return Err(format!("failed validation: {e}"));
            }
            VerifySignatureError::MiniSignError(e) => {
                return Err(format!("failed validation: {e}"));
            }
            VerifySignatureError::SignatureIsNotUtf8 => {
                return Err("failed validation: bad signature (non-utf8)".to_string());
            }
        },
    }

    let mut file = tempfile::Builder::new()
        .prefix("alcom-installer-")
        .suffix(".exe")
        .tempfile()
        .map_err(|e| e.to_string())?;

    // register as a reboot-cleanup safety net immediately —
    // covers crashes/panics/kills between now and normal cleanup below
    if let Err(e) = schedule_delete_on_reboot(file.path()) {
        eprintln!("warning: could not register reboot-cleanup fallback: {e}");
        // non-fatal, just means worse-case cleanup relies solely on the code below
    }

    file.write_all(&response.body).map_err(|e| e.to_string())?;

    // keep() prevents auto-delete-on-drop, since you need the file
    // to survive after this function returns (to CreateProcess it later)

    let path = file.into_temp_path();

    Ok(path)
}

fn display_error_and_exit(window: Option<&winsafe::HWND>, message: &str) -> ! {
    window
        .unwrap_or(&winsafe::HWND::GetDesktopWindow())
        .MessageBox(
            &format!("Error launching ALCOM offline installer: {message}",),
            "ALCOM Online Installer",
            co::MB::OK,
        )
        .ok();
    exit(1);
}

fn schedule_delete_on_reboot(path: &std::path::Path) -> std::io::Result<()> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);

    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
            wide.as_ptr(),
            std::ptr::null(), // null new-name = delete, don't rename
            windows_sys::Win32::Storage::FileSystem::MOVEFILE_DELAY_UNTIL_REBOOT,
        )
    };

    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

mod is_wow64_process_2 {
    use std::ffi::CString;
    use std::mem::transmute;
    use windows_sys::Win32::Foundation::{FALSE, HANDLE};
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
    use windows_sys::Win32::System::SystemInformation::*;
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    use windows_sys::core::BOOL;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HostArch {
        X86,
        X64,
        Arm,
        Arm64,
        Unknown(u16),
    }

    // Signature of IsWow64Process2 (Windows 10 1511+ only)
    type IsWow64Process2Fn = unsafe extern "system" fn(HANDLE, *mut u16, *mut u16) -> BOOL;

    /// Resolves IsWow64Process2 dynamically. Returns None on Win7/8/early Win10
    /// (pre-1511) where the export simply doesn't exist — GetProcAddress fails
    /// gracefully instead of crashing at load time.
    unsafe fn get_is_wow64_process2() -> Option<IsWow64Process2Fn> {
        unsafe {
            // kernel32.dll is always already mapped into the process, so
            // GetModuleHandleA is enough — no LoadLibraryA/FreeLibrary needed.
            let h_module = GetModuleHandleW(windows_sys::w!("kernel32.dll"));
            if h_module == std::ptr::null_mut() {
                return None;
            }

            let fn_name = CString::new("IsWow64Process2").ok()?;
            let proc = GetProcAddress(h_module, fn_name.as_ptr() as *const u8)?;
            Some(transmute::<_, IsWow64Process2Fn>(proc))
        }
    }

    fn machine_to_arch(machine: u16) -> HostArch {
        match machine {
            IMAGE_FILE_MACHINE_AMD64 => HostArch::X64,
            IMAGE_FILE_MACHINE_I386 => HostArch::X86,
            IMAGE_FILE_MACHINE_ARM64 => HostArch::Arm64,
            // IMAGE_FILE_MACHINE_ARMNT: ARM Thumb-2 Little-Endian
            IMAGE_FILE_MACHINE_ARM | IMAGE_FILE_MACHINE_ARMNT => HostArch::Arm,
            other => HostArch::Unknown(other),
        }
    }

    fn processor_arch_to_arch(pa: u16) -> HostArch {
        match pa {
            PROCESSOR_ARCHITECTURE_AMD64 => HostArch::X64,
            PROCESSOR_ARCHITECTURE_INTEL => HostArch::X86,
            PROCESSOR_ARCHITECTURE_ARM64 => HostArch::Arm64,
            PROCESSOR_ARCHITECTURE_ARM => HostArch::Arm,
            other => HostArch::Unknown(other as u16),
        }
    }

    /// Native hardware architecture of the machine, regardless of whether this
    /// (x64) process is itself running under emulation (e.g. x64-on-ARM64).
    pub fn get_host_arch() -> HostArch {
        unsafe {
            if let Some(is_wow64_process2) = get_is_wow64_process2() {
                let mut process_machine: u16 = 0;
                let mut native_machine: u16 = 0;
                let current_process = GetCurrentProcess();

                let ok =
                    is_wow64_process2(current_process, &mut process_machine, &mut native_machine);
                if ok != FALSE {
                    return machine_to_arch(native_machine);
                }
                // if it failed anyway, fall through to the legacy path
            }

            // Windows 7 / 8 / early Windows 10 (pre-1511) path, and general fallback.
            let mut si: SYSTEM_INFO = std::mem::zeroed();
            GetNativeSystemInfo(&mut si);
            let pa = si.Anonymous.Anonymous.wProcessorArchitecture;
            processor_arch_to_arch(pa)
        }
    }
}

mod commandline_param_builder {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::System::Environment::GetCommandLineW;

    /// Returns the raw remainder of the command line after argv[0],
    /// including the leading whitespace, as UTF-16 code units (no null terminator).
    fn raw_args_tail() -> &'static [u16] {
        let cmdline_ptr = unsafe { GetCommandLineW() };

        // find length of the full command line
        let mut len = 0isize;
        while unsafe { *cmdline_ptr.offset(len) } != 0 {
            len += 1;
        }
        let full = unsafe { std::slice::from_raw_parts(cmdline_ptr, len as usize) };

        let mut i = 0usize;
        if full.get(0) == Some(&('"' as u16)) {
            // quoted argv[0]: skip opening quote, scan to matching closing quote
            i += 1;
            while i < full.len() && full[i] != '"' as u16 {
                i += 1;
            }
            if i < full.len() {
                i += 1; // skip closing quote
            }
        } else {
            // unquoted argv[0]: scan to first whitespace
            while i < full.len() && full[i] != ' ' as u16 && full[i] != '\t' as u16 {
                i += 1;
            }
        }

        &full[i..] // everything after argv[0], leading spaces included
    }

    /// Quotes a single argument per the Windows CRT argv-parsing rules.
    fn quote_arg(arg: &OsStr) -> Vec<u16> {
        let wide: Vec<u16> = arg.encode_wide().collect();

        let needs_quotes = wide.is_empty()
            || wide
                .iter()
                .any(|&c| c == ' ' as u16 || c == '\t' as u16 || c == '"' as u16);

        if !needs_quotes {
            return wide;
        }

        let mut out = vec!['"' as u16];
        let mut backslashes = 0usize;

        for &c in &wide {
            if c == '\\' as u16 {
                backslashes += 1;
            } else if c == '"' as u16 {
                // escape all pending backslashes, then escape the quote
                out.extend(std::iter::repeat('\\' as u16).take(backslashes * 2 + 1));
                out.push('"' as u16);
                backslashes = 0;
                continue;
            } else {
                out.extend(std::iter::repeat('\\' as u16).take(backslashes));
                backslashes = 0;
            }
            out.push(c);
        }
        // trailing backslashes must be doubled before the closing quote
        out.extend(std::iter::repeat('\\' as u16).take(backslashes * 2));
        out.push('"' as u16);
        out
    }

    pub fn build_command_line_passthrough(exe_path: &std::path::Path) -> Vec<u16> {
        let mut cmdline = quote_arg(exe_path.as_os_str()); // just argv[0], quoted
        cmdline.extend(raw_args_tail()); // original tail, untouched
        cmdline.push(0); // null terminator for CreateProcessW
        cmdline
    }
}
