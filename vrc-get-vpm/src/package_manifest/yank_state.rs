/*
 * Copyright (c) 2023 anatawa12 and other contributors
 * SPDX-License-Identifier: MPL-2.0
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org.
 */

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::fmt::Formatter;

#[derive(Debug, Clone, Default)]
pub(crate) enum YankState {
    #[default]
    NotYanked,
    NoReason,
    Reason(Box<str>),
}

impl YankState {
    pub fn is_yanked(&self) -> bool {
        match self {
            YankState::NotYanked => false,
            YankState::NoReason => true,
            YankState::Reason(_) => true,
        }
    }

    #[allow(dead_code)]
    pub fn reason(&self) -> Option<&str> {
        match self {
            YankState::Reason(s) => Some(s),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for YankState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VisitorImpl;
        impl serde::de::Visitor<'_> for VisitorImpl {
            type Value = YankState;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean or a string")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if v {
                    Ok(YankState::NoReason)
                } else {
                    Ok(YankState::NotYanked)
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(YankState::Reason(v.into()))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(YankState::Reason(v.into()))
            }
        }

        deserializer.deserialize_any(VisitorImpl)
    }
}
