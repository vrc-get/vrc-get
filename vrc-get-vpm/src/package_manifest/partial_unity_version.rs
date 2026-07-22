use std::fmt::Display;
use std::str::FromStr;

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

#[derive(Debug)]
pub struct InvalidPartialUnityVersion;

impl Display for InvalidPartialUnityVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Invalid unity version")
    }
}

impl std::error::Error for InvalidPartialUnityVersion {}

impl FromStr for PartialUnityVersion {
    type Err = InvalidPartialUnityVersion;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((maj, min)) = s.split_once('.') {
            let major = (maj.trim().parse::<u16>()).map_err(|_| InvalidPartialUnityVersion)?;
            let minor = (min.trim().parse::<u8>()).map_err(|_| InvalidPartialUnityVersion)?;
            Ok(PartialUnityVersion(major, minor))
        } else {
            let major = (s.trim().parse::<u16>()).map_err(|_| InvalidPartialUnityVersion)?;
            Ok(PartialUnityVersion(major, 0))
        }
    }
}
