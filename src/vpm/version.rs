use semver::Version;
use serde::de;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Formatter;
use std::str::FromStr;

// TODO: implement struct
#[derive(Debug, Clone)]
pub struct VersionRange {
    _buffer: String,
}

impl VersionRange {
    pub(crate) fn matches(&self, p0: &Version) -> bool {
        true
    }
}

impl FromStr for VersionRange {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            _buffer: s.to_owned(),
        })
    }
}

impl Serialize for VersionRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self._buffer)
    }
}

impl<'de> Deserialize<'de> for VersionRange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> de::Visitor<'de> for Visitor {
            type Value = VersionRange;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("version range")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(VersionRange {
                    _buffer: v.to_owned(),
                })
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
