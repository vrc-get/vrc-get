// macOS-specific functionality.

use cocoa::foundation::NSProcessInfo;

pub(super) fn compute_os_info() -> String {
    unsafe {
        let process_info = NSProcessInfo::processInfo(std::ptr::null_mut());
        let os_info = NSProcessInfo::operatingSystemVersion(process_info);

        format!(
            "macOS {}.{}.{}",
            os_info.majorVersion, os_info.minorVersion, os_info.patchVersion,
        )
    }
}

pub use open::that as open_that;
use std::process::Command;

pub fn initialize(_: tauri::AppHandle) {
    // nothing to initialize
}

pub(crate) fn fix_env_variables(_: &mut Command) {
    // nothing to do
}
