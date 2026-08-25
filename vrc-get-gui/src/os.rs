//! OS-specific functionality.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum BringUnityToFrontResult {
    BroughtToFront,
    FailedToBringToFront,
    WindowNotFound,
    Unsupported,
}

#[cfg(windows)]
#[path = "os_windows.rs"]
mod platform;

#[cfg(not(windows))]
#[path = "os_posix.rs"]
mod platform;

pub(crate) use platform::*;
