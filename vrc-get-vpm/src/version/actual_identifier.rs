use crate::version::identifier::Identifier;
use crate::version::{FromParsingBuf, ParseVersionError, ParsingBuf};
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::str::FromStr;

/// Optional pre-release identifier on a version string.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct Prerelease {
    identifier: Identifier,
}

from_str_impl!(Prerelease);

impl Prerelease {
    pub const EMPTY: Self = Self {
        identifier: Identifier::EMPTY,
    };

    pub fn new(text: &str) -> Result<Self, ParseVersionError> {
        Prerelease::from_str(text)
    }

    pub fn as_str(&self) -> &str {
        self.identifier.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.identifier.is_empty()
    }
}

impl Display for Prerelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Debug for Prerelease {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Prerelease({})", self.as_str())
    }
}

impl FromParsingBuf for Prerelease {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
        let text = parse_id(buffer, false)?;
        Ok(Prerelease {
            identifier: Identifier::new(text),
        })
    }
}

impl PartialOrd for Prerelease {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

impl Ord for Prerelease {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match self.is_empty() {
            true if rhs.is_empty() => return Ordering::Equal,
            // A real releases are greater than prerelease.
            true => return Ordering::Greater,
            // Prerelease are less than the real release.
            false if rhs.is_empty() => return Ordering::Less,
            false => {}
        }

        let lhs = self.as_str().split('.');
        let mut rhs = rhs.as_str().split('.');

        for lhs in lhs {
            let rhs = match rhs.next() {
                // shorter releases are greater
                None => return Ordering::Greater,
                Some(rhs) => rhs,
            };

            let ordering = match (
                lhs.bytes().all(|b| b.is_ascii_digit()),
                rhs.bytes().all(|b| b.is_ascii_digit()),
            ) {
                // numeric ordering
                (true, true) => Ord::cmp(&lhs.len(), &rhs.len()).then_with(|| Ord::cmp(lhs, rhs)),
                // Numeric identifiers always have lower precedence than non-numeric identifiers.
                (true, false) => return Ordering::Less,
                (false, true) => return Ordering::Greater,
                // Identifiers with letters or hyphens are compared lexically in ASCII sort order.
                (false, false) => Ord::cmp(lhs, rhs),
            };

            if ordering != Ordering::Equal {
                return ordering;
            }
        }

        if rhs.next().is_none() {
            Ordering::Equal
        } else {
            // shorter releases are greater
            Ordering::Less
        }
    }
}

/// Optional pre-release identifier on a version string.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct BuildMetadata {
    identifier: Identifier,
}

from_str_impl!(BuildMetadata);

impl BuildMetadata {
    pub const EMPTY: Self = Self {
        identifier: Identifier::EMPTY,
    };

    pub fn new(text: &str) -> Result<Self, ParseVersionError> {
        BuildMetadata::from_str(text)
    }

    pub fn as_str(&self) -> &str {
        self.identifier.as_str()
    }

    pub fn is_empty(&self) -> bool {
        self.identifier.is_empty()
    }
}

impl Display for BuildMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Debug for BuildMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuildMetadata({})", self.as_str())
    }
}

impl FromParsingBuf for BuildMetadata {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
        let text = parse_id(buffer, true)?;
        Ok(Self {
            identifier: Identifier::new(text),
        })
    }
}

fn parse_id<'a>(
    bytes: &mut ParsingBuf<'a>,
    allow_loading_zero: bool,
) -> Result<&'a str, ParseVersionError> {
    let buf = bytes.buf;
    'outer: loop {
        let mut leading_zero = false;
        let mut alphanumeric = false;
        match bytes.first() {
            None => return Err(ParseVersionError::unexpected_end()),
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
            Some(b'.') => return Err(ParseVersionError::invalid()),
            _ => return Err(ParseVersionError::invalid()),
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
                        return Err(ParseVersionError::invalid());
                    }

                    break 'segment;
                }
                _ => {
                    // end of segment
                    if !allow_loading_zero && alphanumeric && leading_zero {
                        // leading zero is invalid char
                        return Err(ParseVersionError::invalid());
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
