use std::fmt;
use std::str::FromStr;
use serde::{Serialize, Serializer};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnityVersion {
    // major version such as 2019, 2022, and 6
    // note: 5 < 2017 < 2023 < 6 < 7 ...
    // this system assumes if this value is greater than 2000, it's year release
    major: u16,
    // minor version of unity. usually 1, 2 for tech and 3 for LTS
    minor: u8,
    // revision number, updated every few weeks
    revision: u8,
    // release type. a < b < f = c < p < x
    //   'a' for alpha (expects revision to be zero)
    //   'b' for beta (expects revision to be zero)
    //   'f' and 'c' for normal ('c' is for china)
    //   'p' for patches
    //   'x' for experimental
    type_: ReleaseType,
    // revision increment
    increment: u8,
}

impl UnityVersion {
    pub fn new(major: u16, minor: u8, revision: u8, type_: ReleaseType, increment: u8) -> Self {
        Self {
            major,
            minor,
            revision,
            type_,
            increment,
        }
    }

    // expects major.minor.revision[type]increment
    pub fn parse(input: &str) -> Option<Self> {
        let (major, rest) = input.split_once('.')?;
        let major = u16::from_str(major).ok()?;
        let (minor, rest) = rest.split_once('.')?;
        let minor = u8::from_str(minor).ok()?;
        let revision_delimiter = rest.find(is_release_type_char)?;
        let revision = &rest[..revision_delimiter];
        let revision = u8::from_str(revision).ok()?;
        let type_ = ReleaseType::try_from(rest.as_bytes()[revision_delimiter]).ok()?;
        let rest = &rest[revision_delimiter + 1..];

        let (increment, _rest) = rest.split_once('-').unwrap_or((rest, ""));
        let increment = u8::from_str(increment).ok()?;

        return Some(Self {
            major,
            minor,
            revision,
            type_,
            increment,
        });

        fn is_release_type_char(c: char) -> bool {
            c == 'a' || c == 'b' || c == 'f' || c == 'c' || c == 'p' || c == 'x'
        }
    }

    pub fn major(self) -> u16 {
        self.major
    }

    pub fn minor(self) -> u8 {
        self.minor
    }
}

impl fmt::Display for UnityVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{maj}.{min}.{rev}{ty}{inc}",
            maj = self.major,
            min = self.minor,
            rev = self.revision,
            ty = self.type_,
            inc = self.increment,
        )
    }
}

impl Serialize for UnityVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseType {
    Alpha,
    Beta,
    Normal,
    China,
    Patch,
    Experimental,
}

pub struct ReleaseTypeError(());

impl fmt::Display for ReleaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReleaseType::Alpha => f.write_str("a"),
            ReleaseType::Beta => f.write_str("b"),
            ReleaseType::Normal => f.write_str("f"),
            ReleaseType::China => f.write_str("c"),
            ReleaseType::Patch => f.write_str("p"),
            ReleaseType::Experimental => f.write_str("x"),
        }
    }
}

impl TryFrom<u8> for ReleaseType {
    type Error = ReleaseTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'a' => Ok(Self::Alpha),
            b'b' => Ok(Self::Beta),
            b'f' => Ok(Self::Normal),
            b'c' => Ok(Self::China),
            b'p' => Ok(Self::Patch),
            b'x' => Ok(Self::Experimental),
            _ => Err(ReleaseTypeError(())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unity_version() {
        macro_rules! good {
            ($string: literal, $major: literal, $minor: literal, $revision: literal, $type_: ident, $increment: literal) => {
                let version = UnityVersion::parse($string).unwrap();
                assert_eq!(version.major, $major);
                assert_eq!(version.minor, $minor);
                assert_eq!(version.revision, $revision);
                assert_eq!(version.type_, ReleaseType::$type_);
                assert_eq!(version.increment, $increment);
            };
        }

        macro_rules! bad {
            ($string: literal) => {
                assert!(UnityVersion::parse($string).is_none());
            };
        }

        good!("5.6.6f1", 5, 6, 6, Normal, 1);

        good!("2019.1.0a1", 2019, 1, 0, Alpha, 1);
        good!("2019.1.0b1", 2019, 1, 0, Beta, 1);
        good!("2019.4.31f1", 2019, 4, 31, Normal, 1);
        good!("2023.3.6f1", 2023, 3, 6, Normal, 1);
        good!("2023.3.6c1", 2023, 3, 6, China, 1);
        good!("2023.3.6p1", 2023, 3, 6, Patch, 1);
        good!("2023.3.6x1", 2023, 3, 6, Experimental, 1);

        good!("2019.1.0a1-EXTRA", 2019, 1, 0, Alpha, 1);

        bad!("2022");
        bad!("2019.0");
        bad!("5.6.6");
        bad!("2023.4.6f");
    }
}
