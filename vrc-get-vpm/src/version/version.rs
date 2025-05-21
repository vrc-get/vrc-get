use crate::version::parsing_buf::{FromParsingBuf, ParseVersionError, ParsingBuf};
use crate::version::segment::Segment;
use crate::version::{BuildMetadata, Prerelease};
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Write};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// custom version implementation to avoid compare build meta
#[derive(Debug, Clone)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Prerelease,
    pub build: BuildMetadata,
}

from_str_impl!(Version);
serialize_to_string!(Version);
deserialize_from_str!(Version, "version");

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;

        if !self.pre.is_empty() {
            f.write_char('-')?;
            Display::fmt(&self.pre, f)?;
        }

        if !self.build.is_empty() {
            f.write_char('+')?;
            Display::fmt(&self.build, f)?;
        }

        Ok(())
    }
}

impl FromParsingBuf for Version {
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
        let major = parse_segment(bytes)?;
        bytes.read('.')?;
        let minor = parse_segment(bytes)?;
        bytes.read('.')?;
        let patch = parse_segment(bytes)?;

        let pre = if let Some(b'-') = bytes.first() {
            bytes.skip();
            Prerelease::parse(bytes)?
        } else {
            Prerelease::EMPTY
        };

        let build = if let Some(b'+') = bytes.first() {
            bytes.skip();
            BuildMetadata::parse(bytes)?
        } else {
            BuildMetadata::EMPTY
        };

        return Ok(Version {
            major,
            minor,
            patch,
            pre,
            build,
        });

        fn parse_segment(bytes: &mut ParsingBuf) -> Result<u64, ParseVersionError> {
            match bytes.first() {
                Some(b'1'..=b'9') => {
                    let mut i = 1;
                    while let Some(b'0'..=b'9') = bytes.get(i) {
                        i += 1;
                    }
                    let str = bytes.take(i);
                    let value = Segment::from_str(str)
                        .map_err(|_| ParseVersionError::too_big())?
                        .as_number()
                        .unwrap();
                    Ok(value)
                }
                Some(b'0') => {
                    bytes.skip();
                    // if 0\d, 0 is invalid char
                    if let Some(b'0'..=b'9') = bytes.first() {
                        return Err(ParseVersionError::invalid());
                    }
                    Ok(0)
                }
                Some(_) => Err(ParseVersionError::invalid()),
                None => Err(ParseVersionError::invalid()),
            }
        }
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.major.hash(state);
        self.minor.hash(state);
        self.patch.hash(state);
        self.pre.hash(state);
    }
}

impl PartialEq<Self> for Version {
    fn eq(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch == other.patch
            && self.pre == other.pre
    }
}

impl Eq for Version {}

impl PartialOrd<Self> for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then_with(|| self.minor.cmp(&other.minor))
            .then_with(|| self.patch.cmp(&other.patch))
            .then_with(|| self.pre.cmp(&other.pre))
    }
}

impl Version {
    pub fn new(major: u64, minor: u64, patch: u64) -> Version {
        Version {
            major,
            minor,
            patch,
            pre: Prerelease::EMPTY,
            build: BuildMetadata::EMPTY,
        }
    }

    pub fn new_pre(major: u64, minor: u64, patch: u64, pre: Prerelease) -> Version {
        Version {
            major,
            minor,
            patch,
            pre,
            build: BuildMetadata::EMPTY,
        }
    }

    pub fn base_version(&self) -> Version {
        Version::new(self.major, self.minor, self.patch)
    }

    pub fn is_pre(&self) -> bool {
        !self.is_stable()
    }

    pub fn is_stable(&self) -> bool {
        self.pre.is_empty()
    }
}

pub struct StrictEqVersion<V: Borrow<Version> = Version>(pub V);

impl<V1, V2> PartialEq<StrictEqVersion<V2>> for StrictEqVersion<V1>
where
    V1: Borrow<Version>,
    V2: Borrow<Version>,
{
    fn eq(&self, other: &StrictEqVersion<V2>) -> bool {
        let this = self.0.borrow();
        let other = other.0.borrow();
        this.major == other.major
            && this.minor == other.minor
            && this.patch == other.patch
            && this.pre == other.pre
            && this.build == other.build
    }
}

impl Eq for StrictEqVersion {}
