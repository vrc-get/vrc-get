/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use crate::io;
use crate::io::DefaultEnvironmentIo;
use crate::utils::json::{JsonArray, JsonError, JsonObject, JsonValue, save_json, try_load_json};

/// since this file is vrc-get specific, additional keys can be removed
#[derive(Debug, Default, Clone)]
struct AsJson {
    ignore_official_repository: bool,
    ignore_curated_repository: bool,
    #[cfg(feature = "experimental-project-management")]
    #[serde(default)]
    project_list_sync_mode: super::project_management::SyncWithLitedbMode,

    raw: JsonObject,
}

impl AsJson {
    fn from_json_value(value: JsonValue) -> Result<Self, JsonError> {
        let object = value.into_object()?;
        Ok(Self {
            ignore_official_repository: object
                .get_opt("ignoreOfficialRepository")
                .try_map(JsonValue::into_bool)?
                .unwrap_or(false),
            ignore_curated_repository: object
                .get_opt("ignoreCuratedRepository")
                .try_map(JsonValue::into_bool)?
                .unwrap_or(false),
            raw: object,
        })
    }

    fn to_json_value(&self) -> JsonValue {
        let mut object = self.raw.clone();
        object.insert("ignoreOfficialRepository", self.ignore_official_repository);
        object.insert("ignoreCuratedRepository", self.ignore_curated_repository);
        object.into()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct VrcGetSettings {
    parsed: AsJson,
    should_save: bool,
}

const JSON_PATH: &str = "vrc-get/settings.json";

impl VrcGetSettings {
    pub async fn load(io: &DefaultEnvironmentIo) -> io::Result<Self> {
        let parsed = if let Some(value) =
            try_load_json(io, JSON_PATH.as_ref(), AsJson::from_json_value).await?
        {
            value
        } else {
            Default::default()
        };

        Ok(Self {
            parsed,
            should_save: false,
        })
    }

    pub fn ignore_official_repository(&self) -> bool {
        self.parsed.ignore_official_repository
    }

    pub fn ignore_curated_repository(&self) -> bool {
        self.parsed.ignore_curated_repository
    }

    #[cfg(feature = "experimental-project-management")]
    pub fn project_list_sync_mode(&self) -> super::project_management::SyncWithLitedbMode {
        self.parsed.project_list_sync_mode
    }

    #[cfg(feature = "experimental-project-management")]
    pub fn set_project_list_sync_mode(
        &mut self,
        mode: super::project_management::SyncWithLitedbMode,
    ) {
        self.parsed.project_list_sync_mode = mode;
        self.should_save = true;
    }

    pub async fn save(&self, io: &DefaultEnvironmentIo) -> io::Result<()> {
        if self.should_save {
            save_json(io, JSON_PATH.as_ref(), self.parsed.to_json_value()).await?;
        }
        Ok(())
    }
}
