/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

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
