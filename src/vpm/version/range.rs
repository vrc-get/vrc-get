use crate::vpm::version::ParsingBuf;
use semver::{BuildMetadata, Prerelease, Version};
use serde::de;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;

// TODO: TEST

// TODO: implement struct
#[derive(Debug, Clone)]
pub struct VersionRange {
    comparators: Vec<ComparatorSet>,
}

impl VersionRange {
    pub(crate) fn matches(&self, version: &Version) -> bool {
        self.comparators.iter().any(|x| x.matches(version))
    }
}

impl Serialize for VersionRange {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
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
                VersionRange::from_str(v).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

impl Display for VersionRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.comparators.iter();
        match iter.next() {
            None => return Ok(()),
            Some(next) => Display::fmt(next, f)?,
        }
        while let Some(next) = iter.next() {
            f.write_str(" || ")?;
            Display::fmt(next, f)?
        }
        Ok(())
    }
}

impl FromStr for VersionRange {
    type Err = ParseRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            comparators: s
                .split("||")
                .map(FromStr::from_str)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone)]
struct ComparatorSet(Vec<Comparator>);

impl Display for ComparatorSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        match iter.next() {
            None => return Ok(()),
            Some(next) => Display::fmt(next, f)?,
        }
        while let Some(next) = iter.next() {
            f.write_str(" ")?;
            Display::fmt(next, f)?
        }
        Ok(())
    }
}

impl FromStr for ComparatorSet {
    type Err = ParseRangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut result = Vec::<Comparator>::new();
        let mut buffer = ParsingBuf::new(s);

        while !buffer.is_empty() {
            result.push(Comparator::parse(&mut buffer)?);
        }

        Ok(Self(result))
    }
}

impl ComparatorSet {
    fn matches(&self, version: &Version) -> bool {
        self.0.iter().all(|x| x.matches(version))
    }
}

#[derive(Debug, Clone)]
enum Comparator {
    Tilde(PartialVersion),
    Caret(PartialVersion),
    Exact(PartialVersion),
    // >
    GreaterThan(PartialVersion),
    GreaterThanOrEqual(PartialVersion),
    // <
    LessThan(PartialVersion),
    LessThanOrEqual(PartialVersion),

    Hyphen(PartialVersion, PartialVersion),

    // without operator
    Star(PartialVersion),
}

impl Display for Comparator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Comparator::Tilde(v) => write!(f, "~{v}"),
            Comparator::Caret(v) => write!(f, "^{v}"),
            Comparator::Exact(v) => write!(f, "={v}"),
            Comparator::GreaterThan(v) => write!(f, ">{v}"),
            Comparator::GreaterThanOrEqual(v) => write!(f, ">={v}"),
            Comparator::LessThan(v) => write!(f, "<{v}"),
            Comparator::LessThanOrEqual(v) => write!(f, "<={v}"),
            Comparator::Hyphen(a, b) => write!(f, "{a}-{b}"),
            Comparator::Star(v) => Display::fmt(v, f),
        }
    }
}

impl Comparator {
    fn matches(&self, version: &Version) -> bool {
        macro_rules! require {
            ($cond: expr) => {
                if !$cond {
                    return false;
                }
            };
        }
        return match self {
            Comparator::Tilde(v) => {
                require!(version.major == v.major_or(0));
                if let Some(minor) = v.minor() {
                    require!(version.minor == minor);
                }
                true
            }
            Comparator::Caret(v) => {
                if !version.pre.is_empty() {
                    // allow x.y.z-prerelease for ^x.y.z
                    if version.major == v.major_or(0)
                        || version.minor == v.minor_or(0)
                        || version.patch == v.patch_or(0)
                    {
                        return true;
                    }
                }
                require!(version.major == v.major_or(0));
                if v.major_or(0) == 0 {
                    // ^0 ^0.x ^0.x.y
                    if let Some(minor) = v.minor() {
                        // ^0.x.y ^0.x
                        require!(version.minor == minor);
                        if let Some(patch) = v.patch() {
                            // ^0.x.y
                            if minor == 0 {
                                // ^0.0.y
                                require!(version.patch == patch);
                            }
                        }
                    }
                }
                true
            }
            Comparator::Star(v) | Comparator::Exact(v) => match full_or_next(v) {
                Ok(full) => &full == version,
                Err(next) => &v.to_zeros() <= version && version < &next,
            },
            Comparator::GreaterThan(v) => greater_than(version, v),
            Comparator::GreaterThanOrEqual(v) => greater_than_or_equal(version, v),
            Comparator::LessThan(v) => less_than(version, v),
            Comparator::LessThanOrEqual(v) => less_than_or_equal(version, v),
            Comparator::Hyphen(lower, upper) => {
                greater_than_or_equal(version, lower) && less_than_or_equal(version, upper)
            }
        };

        fn full_or_next(v: &PartialVersion) -> Result<Version, Version> {
            if let Some(major) = v.major() {
                if let Some(minor) = v.minor() {
                    if let Some(patch) = v.patch() {
                        Ok(Version::new(major, minor, patch))
                    } else {
                        Err(Version::new(major, minor + 1, 0))
                    }
                } else {
                    Err(Version::new(major + 1, 0, 0))
                }
            } else {
                Err(Version::new(Segment::MAX, Segment::MAX, Segment::MAX))
            }
        }

        fn greater_than(version: &Version, v: &PartialVersion) -> bool {
            match full_or_next(v) {
                Ok(full) => version > &full,
                Err(next) => version >= &next,
            }
        }
        fn greater_than_or_equal(version: &Version, v: &PartialVersion) -> bool {
            match v.to_full() {
                Some(v) => version >= &v,
                None => {
                    let zeros = v.to_zeros();
                    version >= &zeros || !version.pre.is_empty() && base_version(version) == zeros
                }
            }
        }
        fn less_than(version: &Version, v: &PartialVersion) -> bool {
            return match v.to_full() {
                Some(v) => version >= &v,
                None => {
                    let zeros = v.to_zeros();
                    version < &zeros || !(!version.pre.is_empty() && base_version(version) == zeros)
                }
            };
        }

        fn less_than_or_equal(version: &Version, v: &PartialVersion) -> bool {
            match full_or_next(v) {
                Ok(full) => version <= &full,
                Err(next) => version < &next,
            }
        }
    }

    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        bytes.slip_ws();
        match bytes.first() {
            Some(b'~') => Ok(Self::Tilde(PartialVersion::parse(bytes.skip())?)),
            Some(b'^') => Ok(Self::Caret(PartialVersion::parse(bytes.skip())?)),
            Some(b'=') => Ok(Self::Exact(PartialVersion::parse(bytes.skip())?)),
            Some(b'>') => {
                bytes.skip();
                if matches!(bytes.first(), Some(b'=')) {
                    bytes.skip();
                    bytes.slip_ws();
                    Ok(Self::GreaterThanOrEqual(PartialVersion::parse(bytes)?))
                } else {
                    bytes.slip_ws();
                    Ok(Self::GreaterThan(PartialVersion::parse(bytes)?))
                }
            }
            Some(b'<') => {
                bytes.skip();
                if matches!(bytes.first(), Some(b'=')) {
                    bytes.skip();
                    bytes.slip_ws();
                    Ok(Self::LessThanOrEqual(PartialVersion::parse(bytes)?))
                } else {
                    bytes.slip_ws();
                    Ok(Self::LessThan(PartialVersion::parse(bytes)?))
                }
            }
            Some(_) => {
                let first = PartialVersion::parse(bytes)?;

                bytes.slip_ws();
                if matches!(bytes.first(), Some(b'-')) {
                    // x.y.z - x.y.z

                    bytes.slip_ws();
                    let second = PartialVersion::parse(bytes.skip())?;
                    Ok(Self::Hyphen(first, second))
                } else {
                    // x.y.z
                    Ok(Self::Star(first))
                }
            }
            None => Err(ParseRangeError::unexpected_end()),
        }
    }
}

type Segment = u64;

const NOT_EXISTS: Segment = Segment::MAX;
const STAR: Segment = NOT_EXISTS - 1;
const UPPER_X: Segment = STAR - 1;
const LOWER_X: Segment = UPPER_X - 1;
const VERSION_SEGMENT_MAX: Segment = LOWER_X - 1;

#[derive(Debug, Clone)]
struct PartialVersion {
    // MAX_VALUE for not exists
    major: Segment,
    // MAX_VALUE for not exists
    minor: Segment,
    // MAX_VALUE for not exists
    patch: Segment,
    pre: Prerelease,
    build: BuildMetadata,
}

impl Display for PartialVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn write_segment(f: &mut Formatter<'_>, value: Segment, prefix: &str) -> std::fmt::Result {
            if value != NOT_EXISTS {
                f.write_str(prefix)?;
                match value {
                    STAR => f.write_char('*')?,
                    UPPER_X => f.write_char('X')?,
                    LOWER_X => f.write_char('x')?,
                    _ => Display::fmt(&value, f)?,
                }
            }
            Ok(())
        }

        write_segment(f, self.major, "")?;
        write_segment(f, self.minor, ".")?;
        write_segment(f, self.patch, ".")?;

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

fn base_version(version: &Version) -> Version {
    Version::new(version.major, version.minor, version.patch)
}

impl PartialVersion {
    fn to_full(&self) -> Option<Version> {
        if let (Some(major), Some(minor), Some(patch)) = (self.major(), self.minor(), self.patch())
        {
            Some(Version {
                major,
                minor,
                patch,
                pre: self.pre.clone(),
                build: self.build.clone(),
            })
        } else {
            None
        }
    }

    fn to_zeros(&self) -> Version {
        Version {
            major: self.patch_or(0),
            minor: self.patch_or(0),
            patch: self.patch_or(0),
            pre: self.pre.clone(),
            build: self.build.clone(),
        }
    }

    fn segment(segment: Segment) -> Option<Segment> {
        if segment <= VERSION_SEGMENT_MAX {
            Some(segment)
        } else {
            None
        }
    }

    fn segment_or(segment: Segment, or: Segment) -> Segment {
        if segment <= VERSION_SEGMENT_MAX {
            segment
        } else {
            or
        }
    }

    fn major(&self) -> Option<Segment> {
        Self::segment(self.major)
    }

    fn major_or(&self, default: Segment) -> Segment {
        Self::segment_or(self.major, default)
    }

    fn minor(&self) -> Option<Segment> {
        Self::segment(self.minor)
    }

    fn minor_or(&self, default: Segment) -> Segment {
        Self::segment_or(self.minor, default)
    }

    fn patch(&self) -> Option<Segment> {
        Self::segment(self.patch)
    }

    fn patch_or(&self, default: Segment) -> Segment {
        Self::segment_or(self.patch, default)
    }

    pub(super) fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        bytes.slip_ws();
        let major = Self::parse_segment(bytes)?;
        let minor = if let Some(b'.') = bytes.first() {
            bytes.skip();
            Self::parse_segment(bytes)?
        } else {
            NOT_EXISTS
        };
        let (patch, pre, build) = if let Some(b'.') = bytes.first() {
            bytes.skip();
            let patch = Self::parse_segment(bytes)?;

            let prerelease = if let Some(b'-') = bytes.first() {
                bytes.skip();
                Prerelease::new(Self::parse_id(bytes, false)?).unwrap()
            } else {
                Prerelease::EMPTY
            };
            let build_meta = if let Some(b'+') = bytes.first() {
                bytes.skip();
                BuildMetadata::new(Self::parse_id(bytes, false)?).unwrap()
            } else {
                BuildMetadata::EMPTY
            };
            (patch, prerelease, build_meta)
        } else {
            (NOT_EXISTS, Prerelease::EMPTY, BuildMetadata::EMPTY)
        };

        Ok(PartialVersion {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }

    fn parse_segment(bytes: &mut ParsingBuf) -> Result<Segment, ParseRangeError> {
        match bytes.first() {
            Some(b'x') => {
                bytes.skip();
                Ok(LOWER_X)
            }
            Some(b'X') => {
                bytes.skip();
                Ok(UPPER_X)
            }
            Some(b'*') => {
                bytes.skip();
                Ok(STAR)
            }
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
}

#[derive(Debug)]
pub struct ParseRangeError {
    inner: Inner,
}

impl Display for ParseRangeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            Inner::VersionSegmentTooBig => f.write_str("version segment too big"),
            Inner::UnexpectedEnd => f.write_str("unexpected end"),
            Inner::InvalidChar(c) => write!(f, "invalid char: {:?}", c),
        }
    }
}

#[derive(Debug)]
enum Inner {
    VersionSegmentTooBig,
    UnexpectedEnd,
    InvalidChar(char),
}

impl ParseRangeError {
    fn too_big() -> Self {
        Self {
            inner: Inner::VersionSegmentTooBig,
        }
    }
    fn invalid_char(c: char) -> Self {
        Self {
            inner: Inner::InvalidChar(c),
        }
    }
    fn unexpected_end() -> ParseRangeError {
        Self {
            inner: Inner::UnexpectedEnd,
        }
    }
}
