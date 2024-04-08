use serde::de::Error;
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
        let s = String::deserialize(deserializer)?;
        if let Some((maj, min)) = s.split_once('.') {
            let major = maj.trim().parse::<u16>().map_err(Error::custom)?;
            let minor = min.trim().parse::<u8>().map_err(Error::custom)?;
            Ok(Self(major, minor))
        } else {
            let major = s.trim().parse::<u16>().map_err(Error::custom)?;
            Ok(Self(major, 0))
        }
    }
}
