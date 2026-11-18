/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::io;
use std::future::Future;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub(crate) struct SaveController<T> {
    parsed: T,
    settings_changed: bool,
}

impl<T> SaveController<T> {
    pub(crate) fn new(parsed: T) -> Self {
        Self {
            parsed,
            settings_changed: false,
        }
    }

    #[inline]
    pub(crate) fn as_mut(&mut self) -> &mut T {
        self.settings_changed = true;
        &mut self.parsed
    }
}

impl<T> SaveController<T> {
    pub(crate) async fn save<'a, F, Fut>(&'a mut self, save: F) -> io::Result<()>
    where
        F: FnOnce(&'a T) -> Fut,
        T: 'a,
        Fut: Future<Output = io::Result<()>> + 'a,
    {
        if !self.settings_changed {
            return Ok(());
        }

        save(&self.parsed).await?;

        self.settings_changed = false;
        Ok(())
    }
}

impl<T> Deref for SaveController<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}
