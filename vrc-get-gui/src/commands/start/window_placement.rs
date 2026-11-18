/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

//! Saving and restoring the main window position.
//!
//! - The position is stored in OS-native desktop coordinates (see [`WindowPosition`]),
//!   like Firefox's session restore.
//! - On restore, the saved position is used only if the title bar is reachable
//!   on one of the current monitors. Otherwise, the OS decides the position,
//!   like tauri-plugin-window-state.

use crate::config::WindowPosition;
use sizes::{NativePosition, NativeRect, NativeSize};
use tauri::{LogicalSize, WebviewWindow};

pub(super) fn get_position(window: &WebviewWindow) -> tauri::Result<Option<WindowPosition>> {
    let position = window.outer_position()?;
    if cfg!(windows) && (position.x <= -32000 || position.y <= -32000) {
        // minimized window on Windows
        return Ok(None);
    }
    let native = NativePosition::from_physical(position, window.scale_factor()?);
    Ok(Some(WindowPosition {
        x: native.x,
        y: native.y,
    }))
}

pub(super) fn restore_position(window: &WebviewWindow, saved: WindowPosition) -> tauri::Result<()> {
    if !saved.x.is_finite() || !saved.y.is_finite() {
        return Ok(());
    }

    let saved_pos = NativePosition::new(saved.x, saved.y);

    let width: f64 = window
        .outer_size()?
        .to_logical::<f64>(window.scale_factor()?)
        .width;

    let title_bar_size = LogicalSize::new(width, 32.0); // we assume title bar height is 32 logical pixels
    let min_visible_size = LogicalSize::new(100.0, 16.0);

    let title_bar_visible = window.available_monitors()?.iter().any(|monitor| {
        let factor = monitor.scale_factor();
        let title_bar =
            NativeRect::new(saved_pos, NativeSize::from_logical(title_bar_size, factor));
        let work_area = NativeRect::from_physical(monitor.work_area(), factor);
        let size = title_bar.intersection_size(&work_area);
        let min_visible_size = NativeSize::from_logical(min_visible_size, factor);
        size.width >= min_visible_size.width && size.height >= min_visible_size.height
    });

    if !title_bar_visible {
        log::info!(
            "saved window position ({}, {}) is not visible on any monitor; leaving it to the OS",
            saved.x,
            saved.y
        );
        return Ok(());
    }

    window.set_position(saved_pos.to_position())
}

mod sizes {
    use tauri::{
        LogicalPosition, LogicalSize, PhysicalPosition, PhysicalRect, PhysicalSize, Pixel, Position,
    };

    #[derive(Copy, Clone, Debug)]
    pub struct NativeRect {
        pub position: NativePosition,
        pub size: NativeSize,
    }

    fn native_to_physical(scale_factor: f64) -> f64 {
        if cfg!(windows) { 1.0 } else { scale_factor }
    }

    fn logical_to_native(scale_factor: f64) -> f64 {
        if cfg!(windows) { scale_factor } else { 1.0 }
    }

    impl NativeRect {
        pub fn new(position: NativePosition, size: NativeSize) -> Self {
            Self { position, size }
        }

        pub fn from_physical<P, S>(rect: &PhysicalRect<P, S>, scale_factor: f64) -> NativeRect
        where
            P: Pixel,
            S: Pixel,
        {
            Self::new(
                NativePosition::from_physical(rect.position, scale_factor),
                NativeSize::from_physical(rect.size, scale_factor),
            )
        }

        pub fn left(&self) -> f64 {
            self.position.x
        }

        pub fn right(&self) -> f64 {
            self.position.x + self.size.width
        }

        pub fn top(&self) -> f64 {
            self.position.y
        }

        pub fn bottom(&self) -> f64 {
            self.position.y + self.size.height
        }

        /// May be negative if they don't intersect.
        pub fn intersection_size(&self, other: &NativeRect) -> NativeSize {
            let w = self.right().min(other.right()) - self.left().max(other.left());
            let h = self.bottom().min(other.bottom()) - self.top().max(other.top());
            NativeSize::new(w, h)
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct NativePosition {
        pub x: f64,
        pub y: f64,
    }

    impl NativePosition {
        pub fn new(x: f64, y: f64) -> Self {
            Self { x, y }
        }

        pub fn from_physical<P: Pixel>(position: PhysicalPosition<P>, scale_factor: f64) -> Self {
            let scale_factor = native_to_physical(scale_factor);
            Self::new(
                position.x.into() / scale_factor,
                position.y.into() / scale_factor,
            )
        }

        #[allow(dead_code)]
        pub fn from_logical<P: Pixel>(position: LogicalPosition<P>, scale_factor: f64) -> Self {
            let scale_factor = logical_to_native(scale_factor);
            Self::new(
                position.x.into() * scale_factor,
                position.y.into() * scale_factor,
            )
        }

        pub fn to_position(self) -> Position {
            if cfg!(windows) {
                Position::Physical(PhysicalPosition::new(
                    self.x.round() as i32,
                    self.y.round() as i32,
                ))
            } else {
                Position::Logical(LogicalPosition::new(self.x, self.y))
            }
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct NativeSize {
        pub width: f64,
        pub height: f64,
    }

    impl NativeSize {
        pub fn new(width: f64, height: f64) -> Self {
            Self { width, height }
        }

        pub fn from_physical<P: Pixel>(size: PhysicalSize<P>, scale_factor: f64) -> Self {
            let scale_factor = native_to_physical(scale_factor);
            Self::new(
                size.width.into() / scale_factor,
                size.height.into() / scale_factor,
            )
        }

        pub fn from_logical<P: Pixel>(size: LogicalSize<P>, scale_factor: f64) -> Self {
            let scale_factor = logical_to_native(scale_factor);
            Self::new(
                size.width.into() * scale_factor,
                size.height.into() * scale_factor,
            )
        }
    }
}
