use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone)]
pub struct PartialUnityVersion(u16, u8);

impl PartialUnityVersion {
    pub fn major(&self) -> u16 {
        self.0
    }

    pub fn minor(&self) -> u8 {
        self.1
    }
}

impl<'de> Deserialize<'de> for PartialUnityVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = PartialUnityVersion;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("unity version (major or major.minor)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                if let Some((maj, min)) = v.split_once('.') {
                    let major = maj
                        .trim()
                        .parse::<u16>()
                        .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
                    let minor = min
                        .trim()
                        .parse::<u8>()
                        .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
                    Ok(PartialUnityVersion(major, minor))
                } else {
                    let major = v
                        .trim()
                        .parse::<u16>()
                        .map_err(|_| Error::invalid_value(Unexpected::Str(v), &self))?;
                    Ok(PartialUnityVersion(major, 0))
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}
