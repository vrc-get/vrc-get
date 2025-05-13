use std::cmp::Ordering;
use std::fmt;
use std::num::NonZeroU8;
use std::str::FromStr;

use crate::version::Version;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
    // for china releases of Unity,
    // we may see f1c1 so we have an additional one increment for china releases
    // For example, https://unity.cn/releases/lts/2022 has 2022.3.22f1c1
    china_increment: Option<NonZeroU8>,
}

impl UnityVersion {
    pub const fn new(
        major: u16,
        minor: u8,
        revision: u8,
        type_: ReleaseType,
        increment: u8,
    ) -> Self {
        Self {
            major,
            minor,
            revision,
            type_,
            increment,
            china_increment: None,
        }
    }

    pub const fn new_f1(major: u16, minor: u8, revision: u8) -> Self {
        Self {
            major,
            minor,
            revision,
            type_: ReleaseType::Normal,
            increment: 1,
            china_increment: None,
        }
    }

    pub const fn new_china(
        major: u16,
        minor: u8,
        revision: u8,
        increment: u8,
        china_increment: NonZeroU8,
    ) -> Self {
        Self {
            major,
            minor,
            revision,
            type_: ReleaseType::China,
            increment,
            china_increment: Some(china_increment),
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

        let (increment_part, _rest) = rest.split_once('-').unwrap_or((rest, ""));
        let (increment, china_increment);
        if increment_part.contains('c') {
            let (increment_str, increment_china_str) = increment_part.split_once('c')?;
            increment = u8::from_str(increment_str).ok()?;
            china_increment = Some(NonZeroU8::from_str(increment_china_str).ok()?);
        } else {
            increment = u8::from_str(increment_part).ok()?;
            china_increment = None;
        }

        return Some(Self {
            major,
            minor,
            revision,
            type_,
            increment,
            china_increment,
        });

        fn is_release_type_char(c: char) -> bool {
            c == 'a' || c == 'b' || c == 'f' || c == 'c' || c == 'p' || c == 'x'
        }
    }

    // expects major.minor.revision
    pub fn parse_no_type_increment(input: &str) -> Option<Self> {
        let (major, rest) = input.split_once('.')?;
        let major = u16::from_str(major).ok()?;
        let (minor, rest) = rest.split_once('.')?;
        let minor = u8::from_str(minor).ok()?;
        let revision = rest;
        let revision = u8::from_str(revision).ok()?;

        Some(Self::new_f1(major, minor, revision))
    }

    pub fn major(self) -> u16 {
        self.major
    }

    pub fn minor(self) -> u8 {
        self.minor
    }

    pub fn revision(self) -> u8 {
        self.revision
    }

    pub fn type_(self) -> ReleaseType {
        self.type_
    }

    pub fn increment(self) -> u8 {
        self.increment
    }

    pub fn china_increment(self) -> Option<NonZeroU8> {
        self.china_increment
    }

    pub fn as_semver(self) -> Version {
        Version::new(self.major as u64, self.minor as u64, self.revision as u64)
    }
}

impl fmt::Display for UnityVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(china) = self.china_increment {
            write!(
                f,
                "{maj}.{min}.{rev}{ty}{inc}c{china}",
                maj = self.major,
                min = self.minor,
                rev = self.revision,
                ty = self.type_,
                inc = self.increment,
                china = china.get(),
            )
        } else {
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
}

impl Serialize for UnityVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UnityVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        UnityVersion::parse(&String::deserialize(deserializer)?)
            .ok_or_else(|| D::Error::custom("invalid unity version"))
    }
}

impl PartialOrd<Self> for UnityVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for UnityVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        // We ignore china increment for comparing version
        (self.major().cmp(&other.major()))
            .then_with(|| self.minor().cmp(&other.minor()))
            .then_with(|| self.revision().cmp(&other.revision()))
            .then_with(|| self.type_().cmp(&other.type_()))
            .then_with(|| self.increment().cmp(&other.increment()))
    }
}

#[derive(Clone, Copy, Debug, Eq)]
pub enum ReleaseType {
    Alpha,
    Beta,
    Normal,
    China,
    Patch,
    Experimental,
}

impl PartialEq for ReleaseType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Alpha, Self::Alpha) => true,
            (Self::Beta, Self::Beta) => true,
            (Self::Normal, Self::Normal) => true,
            (Self::China, Self::China) => true,
            (Self::Patch, Self::Patch) => true,
            (Self::Experimental, Self::Experimental) => true,

            // exceptions!
            (Self::Normal, Self::China) => true,
            (Self::China, Self::Normal) => true,
            _ => false,
        }
    }
}

impl PartialOrd for ReleaseType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for ReleaseType {
    fn cmp(&self, other: &Self) -> Ordering {
        use Ordering::*;
        use ReleaseType::*;
        match (*self, *other) {
            (Alpha, Alpha) => Equal,
            (Alpha, _) => Less,
            (_, Alpha) => Greater,

            (Beta, Beta) => Equal,
            (Beta, _) => Less,
            (_, Beta) => Greater,

            (Normal, Normal) => Equal,
            (Normal, China) => Equal,
            (China, Normal) => Equal,
            (China, China) => Equal,
            (Normal, _) => Less,
            (China, _) => Less,
            (_, Normal) => Greater,
            (_, China) => Greater,

            (Patch, Patch) => Equal,
            (Patch, _) => Less,
            (_, Patch) => Greater,

            (Experimental, Experimental) => Equal,
        }
    }
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
                assert!(matches!(version.type_, ReleaseType::$type_));
                assert_eq!(version.increment, $increment);
                assert_eq!(version.china_increment, None);
            };
        }

        macro_rules! good_cn {
            ($string: literal, $major: literal, $minor: literal, $revision: literal, $type_: ident, $increment: literal, $china_increment: literal) => {
                let version = UnityVersion::parse($string).unwrap();
                assert_eq!(version.major, $major);
                assert_eq!(version.minor, $minor);
                assert_eq!(version.revision, $revision);
                assert!(matches!(version.type_, ReleaseType::$type_));
                assert_eq!(version.increment, $increment);
                assert_eq!(
                    version.china_increment,
                    Some(NonZeroU8::new($china_increment).unwrap())
                );
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

        good_cn!("2022.3.22f1c1", 2022, 3, 22, Normal, 1, 1);

        bad!("2022");
        bad!("2019.0");
        bad!("5.6.6");
        bad!("2023.4.6f");
    }

    #[test]
    fn ord_version() {
        macro_rules! test {
            ($left: literal <  $right: literal) => {
                let left = UnityVersion::parse($left).unwrap();
                let right = UnityVersion::parse($right).unwrap();
                assert!(left < right);
                assert!(right > left);
            };
        }

        test!("5.6.5f1" < "5.6.6f1");
        test!("5.6.6f1" < "5.6.6f2");
        test!("5.6.6f1" < "2022.1.0f1");
        test!("2022.1.0a1" < "2022.1.0f1");
    }

    #[test]
    fn ord_release_type() {
        use ReleaseType::*;

        macro_rules! test {
            ($left: ident $right: ident $ordering: ident) => {
                assert_eq!($left.cmp(&$right), Ordering::$ordering);
            };
        }

        assert!(Alpha < Beta);
        assert!(Beta < Normal);
        assert!(Beta < China);
        assert!(China < Patch);
        assert!(Patch < Experimental);

        test!(Alpha Alpha Equal);
        test!(Alpha Beta Less);
        test!(Alpha Normal Less);
        test!(Alpha China Less);
        test!(Alpha Patch Less);
        test!(Alpha Experimental Less);

        test!(Beta Alpha Greater);
        test!(Beta Beta Equal);
        test!(Beta Normal Less);
        test!(Beta China Less);
        test!(Beta Patch Less);
        test!(Beta Experimental Less);

        test!(Normal Alpha Greater);
        test!(Normal Beta Greater);
        test!(Normal Normal Equal);
        test!(Normal China Equal);
        test!(Normal Patch Less);
        test!(Normal Experimental Less);

        test!(China Alpha Greater);
        test!(China Beta Greater);
        test!(China Normal Equal);
        test!(China China Equal);
        test!(China Patch Less);
        test!(China Experimental Less);

        test!(Patch Alpha Greater);
        test!(Patch Beta Greater);
        test!(Patch Normal Greater);
        test!(Patch China Greater);
        test!(Patch Patch Equal);
        test!(Patch Experimental Less);

        test!(Experimental Alpha Greater);
        test!(Experimental Beta Greater);
        test!(Experimental Normal Greater);
        test!(Experimental China Greater);
        test!(Experimental Patch Greater);
        test!(Experimental Experimental Equal);
    }

    #[test]
    fn eq_release_type() {
        use ReleaseType::*;

        assert_eq!(Alpha, Alpha);
        assert_ne!(Alpha, Beta);
        assert_ne!(Alpha, Normal);
        assert_ne!(Alpha, China);
        assert_ne!(Alpha, Patch);
        assert_ne!(Alpha, Experimental);

        assert_ne!(Beta, Alpha);
        assert_eq!(Beta, Beta);
        assert_ne!(Beta, Normal);
        assert_ne!(Beta, China);
        assert_ne!(Beta, Patch);
        assert_ne!(Beta, Experimental);

        assert_ne!(Normal, Alpha);
        assert_ne!(Normal, Beta);
        assert_eq!(Normal, Normal);
        assert_eq!(Normal, China);
        assert_ne!(Normal, Patch);
        assert_ne!(Normal, Experimental);

        assert_ne!(China, Alpha);
        assert_ne!(China, Beta);
        assert_eq!(China, Normal);
        assert_eq!(China, China);
        assert_ne!(China, Patch);
        assert_ne!(China, Experimental);

        assert_ne!(Patch, Alpha);
        assert_ne!(Patch, Beta);
        assert_ne!(Patch, Normal);
        assert_ne!(Patch, China);
        assert_eq!(Patch, Patch);
        assert_ne!(Patch, Experimental);

        assert_ne!(Experimental, Alpha);
        assert_ne!(Experimental, Beta);
        assert_ne!(Experimental, Normal);
        assert_ne!(Experimental, China);
        assert_ne!(Experimental, Patch);
        assert_eq!(Experimental, Experimental);
    }
}
