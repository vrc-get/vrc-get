/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use indexmap::IndexMap;
use std::path::Path;
use url::Url;

pub(crate) struct RepoSource<'a> {
    cache_path: &'a Path,
    headers: &'a IndexMap<Box<str>, Box<str>>,
    url: Option<&'a Url>,
}

impl<'a> RepoSource<'a> {
    pub fn new(
        cache_path: &'a Path,
        headers: &'a IndexMap<Box<str>, Box<str>>,
        url: Option<&'a Url>,
    ) -> Self {
        Self {
            cache_path,
            headers,
            url,
        }
    }

    pub fn cache_path(&self) -> &Path {
        self.cache_path
    }

    pub fn headers(&self) -> &IndexMap<Box<str>, Box<str>> {
        self.headers
    }

    pub fn url(&self) -> Option<&Url> {
        self.url
    }
}
