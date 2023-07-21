use super::*;
use semver::{BuildMetadata, Prerelease};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter, Write};
use std::hash::Hash;
use std::str::FromStr;

/// custom version implementation to avoid compare build meta
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Version {
    pub major: Segment,
    pub minor: Segment,
    pub patch: Segment,
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
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        bytes.skip_ws();
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

        fn parse_segment(bytes: &mut ParsingBuf) -> Result<Segment, ParseRangeError> {
            match bytes.first() {
                Some(b'1'..=b'9') => {
                    let mut i = 1;
                    while let Some(b'0'..=b'9') = bytes.get(i) {
                        i += 1;
                    }
                    let str = bytes.take(i);
                    let value = Segment::from_str(str).map_err(|_| ParseRangeError::too_big())?;
                    if value > VERSION_SEGMENT_MAX {
                        return Err(ParseRangeError::too_big());
                    }
                    Ok(value)
                }
                Some(b'0') => {
                    bytes.skip();
                    // if 0\d, 0 is invalid char
                    if let Some(b'0'..=b'9') = bytes.first() {
                        return Err(ParseRangeError::invalid_char(bytes.first_char()));
                    }
                    Ok(0)
                }
                Some(_) => Err(ParseRangeError::invalid_char(bytes.first_char())),
                None => Err(ParseRangeError::unexpected_end()),
            }
        }
    }
}

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
    pub fn new(major: Segment, minor: Segment, patch: Segment) -> Version {
        Version {
            major,
            minor,
            patch,
            pre: Prerelease::EMPTY,
            build: BuildMetadata::EMPTY,
        }
    }

    pub fn new_pre(major: Segment, minor: Segment, patch: Segment, pre: Prerelease) -> Version {
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

impl FromParsingBuf for Prerelease {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        Ok(Prerelease::new(parse_id(buffer, false)?).unwrap())
    }
}

impl FromParsingBuf for BuildMetadata {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        Ok(BuildMetadata::new(parse_id(buffer, true)?).unwrap())
    }
}

fn parse_id<'a>(
    bytes: &mut ParsingBuf<'a>,
    allow_loading_zero: bool,
) -> Result<&'a str, ParseRangeError> {
    let buf = bytes.buf;
    'outer: loop {
        let mut leading_zero = false;
        let mut alphanumeric = false;
        match bytes.first() {
            None => return Err(ParseRangeError::unexpected_end()),
            Some(b'0') => {
                bytes.skip();
                leading_zero = true;
            }
            Some(b'0'..=b'9') => {
                bytes.skip();
            }
            Some(b'a'..=b'z' | b'A'..=b'Z' | b'-') => {
                bytes.skip();
                alphanumeric = true;
            }
            Some(b'.') => return Err(ParseRangeError::invalid_char('.')),
            _ => return Err(ParseRangeError::invalid_char(bytes.first_char())),
        }
        'segment: loop {
            match bytes.first() {
                Some(b'0'..=b'9') => {
                    bytes.skip();
                }
                Some(b'a'..=b'z' | b'A'..=b'Z' | b'-') => {
                    bytes.skip();
                    alphanumeric = true;
                }
                Some(b'.') => {
                    bytes.skip();
                    if !allow_loading_zero && alphanumeric && leading_zero {
                        // leading zero is invalid char
                        return Err(ParseRangeError::invalid_char('0'));
                    }

                    break 'segment;
                }
                _ => {
                    // end of segment
                    if !allow_loading_zero && alphanumeric && leading_zero {
                        // leading zero is invalid char
                        return Err(ParseRangeError::invalid_char('0'));
                    }
                    break 'outer;
                }
            }
        }
    }

    if bytes.buf.len() == 0 {
        Ok(buf)
    } else {
        let len = bytes.buf.as_ptr() as usize - buf.as_ptr() as usize;
        Ok(&buf[..len])
    }
}
