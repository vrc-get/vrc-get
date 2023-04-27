use crate::version::*;
use semver::{BuildMetadata, Prerelease};
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;

// TODO: TEST

// TODO: implement struct
#[derive(Debug, Clone)]
pub struct VersionRange {
    comparators: Vec<ComparatorSet>,
}

impl VersionRange {
    pub fn same_or_later(version: Version) -> Self {
        Self {
            comparators: vec![
                ComparatorSet(vec![
                    Comparator::GreaterThanOrEqual(PartialVersion::from(version))
                ])
            ]
        }
    }

    pub(crate) fn matches(&self, version: &Version) -> bool {
        self.match_pre(version, false)
    }

    pub(crate) fn match_pre(&self, version: &Version, allow_prerelease: bool) -> bool {
        self.comparators.iter().any(|x| x.matches(version, allow_prerelease))
    }
}

serialize_to_string!(VersionRange);
deserialize_from_str!(VersionRange, "version range");

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

from_str_impl!(ComparatorSet);

impl FromParsingBuf for ComparatorSet {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        let mut result = Vec::<Comparator>::new();

        while !buffer.is_empty() {
            result.push(Comparator::parse(buffer)?);
            buffer.skip_ws();
        }

        Ok(Self(result))
    }
}

impl ComparatorSet {
    fn matches(&self, version: &Version, allow_prerelease: bool) -> bool {
        self.0.iter().all(|x| x.matches(version, allow_prerelease))
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
    fn matches(&self, version: &Version, allow_prerelease: bool) -> bool {
        if !self.matches_internal(version) {
            return false;
        }

        macro_rules! allow {
            ($cond: expr) => {
                if $cond {
                    return true;
                }
            };
        }

        allow!(allow_prerelease || version.is_stable());

        // might be prerelease & prerelease is not allowed: check for version existence

        let in_version = match self {
            Self::Tilde(c) => c,
            Self::Caret(c) => c,
            Self::Exact(c) => c,
            Self::GreaterThan(c) => c,
            Self::GreaterThanOrEqual(c) => c,
            Self::LessThan(c) => c,
            Self::LessThanOrEqual(c) => c,
            Self::Star(c) => c,
            Self::Hyphen(c, _) => c,
        };
        let in_version = in_version.to_zeros();

        allow!(in_version.is_pre() && in_version.base_version() == version.base_version());

        // for Hyphen, we have two versions 
        if let Self::Hyphen(_, in_version) = self {
            let in_version = in_version.to_zeros();

            allow!(in_version.is_pre() && in_version.base_version() == version.base_version());
        }

        return false;
    }

    fn matches_internal(&self, version: &Version) -> bool {
        macro_rules! require {
            ($cond: expr) => {
                if !$cond {
                    return false;
                }
            };
        }
        return match self {
            Comparator::Tilde(v) => {
                require!(version >= &v.to_zeros());
                require!(version.major == v.major_or(0));
                if let Some(minor) = v.minor() {
                    require!(version.minor == minor);
                }
                true
            }
            Comparator::Caret(v) => {
                require!(version >= &v.to_zeros());
                // ^* is always true
                if let None = v.major() {
                    return true;
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
                Ok(full) => full.cmp(version).is_eq(),
                Err(next) => {
                    &v.to_zeros() <= version && version < &next
                }
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
                        Ok(Version::new_pre(major, minor, patch, v.pre.clone()))
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
                None => version >= &v.to_zeros(),
            }
        }
        fn less_than(version: &Version, v: &PartialVersion) -> bool {
            return match v.to_full() {
                Some(v) => version < &v,
                None => version < &v.to_zeros(),
            };
        }

        fn less_than_or_equal(version: &Version, v: &PartialVersion) -> bool {
            match full_or_next(v) {
                Ok(full) => version <= &full,
                Err(next) => version < &next,
            }
        }
    }
}

impl FromParsingBuf for Comparator {
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        bytes.skip_ws();
        match bytes.first() {
            Some(b'~') => Ok(Self::Tilde(PartialVersion::parse(bytes.skip())?)),
            Some(b'^') => Ok(Self::Caret(PartialVersion::parse(bytes.skip())?)),
            Some(b'=') => Ok(Self::Exact(PartialVersion::parse(bytes.skip())?)),
            Some(b'>') => {
                bytes.skip();
                if matches!(bytes.first(), Some(b'=')) {
                    bytes.skip();
                    bytes.skip_ws();
                    Ok(Self::GreaterThanOrEqual(PartialVersion::parse(bytes)?))
                } else {
                    bytes.skip_ws();
                    Ok(Self::GreaterThan(PartialVersion::parse(bytes)?))
                }
            }
            Some(b'<') => {
                bytes.skip();
                if matches!(bytes.first(), Some(b'=')) {
                    bytes.skip();
                    bytes.skip_ws();
                    Ok(Self::LessThanOrEqual(PartialVersion::parse(bytes)?))
                } else {
                    bytes.skip_ws();
                    Ok(Self::LessThan(PartialVersion::parse(bytes)?))
                }
            }
            Some(_) => {
                let first = PartialVersion::parse(bytes)?;

                bytes.skip_ws();
                if matches!(bytes.first(), Some(b'-')) {
                    // x.y.z - x.y.z

                    bytes.skip_ws();
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
            major: self.major_or(0),
            minor: self.minor_or(0),
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
}

impl FromParsingBuf for PartialVersion {
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseRangeError> {
        bytes.skip_ws();
        // allow v1.2.3
        if let Some(b'v') = bytes.first() {
            bytes.skip();
        }
        let major = parse_segment(bytes)?;
        let minor = if let Some(b'.') = bytes.first() {
            bytes.skip();
            parse_segment(bytes)?
        } else {
            NOT_EXISTS
        };
        let (patch, pre, build) = if let Some(b'.') = bytes.first() {
            bytes.skip();
            let patch = parse_segment(bytes)?;

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

        return Ok(PartialVersion {
            major,
            minor,
            patch,
            pre,
            build,
        });

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
    }
}

impl PartialVersion {
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

impl From<Version> for PartialVersion {
    fn from(value: Version) -> Self {
        Self {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
            pre: value.pre,
            build: value.build,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_positive() {
        fn test(range: &str, version: &str) {
            let range = VersionRange::from_str(range).expect(range);
            let version = Version::from_str(version).expect(version);
            assert!(range.matches(&version), "{} matches {}", range, version);
        }
        // test set are from node-semver
        // Copyright (c) Isaac Z. Schlueter and Contributors
        // Originally under The ISC License
        // https://github.com/npm/node-semver/blob/3a8a4309ae986c1967b3073ba88c9e69433d44cb/test/fixtures/range-include.js

        test("1.0.0 - 2.0.0", "1.2.3");
        test("^1.2.3+build", "1.2.3");
        test("^1.2.3+build", "1.3.0");
        test("1.2.3-pre+asdf - 2.4.3-pre+asdf", "1.2.3");
        //test("1.2.3pre+asdf - 2.4.3-pre+asdf", "1.2.3", true);
        //test("1.2.3-pre+asdf - 2.4.3pre+asdf", "1.2.3", true);
        //test("1.2.3pre+asdf - 2.4.3pre+asdf", "1.2.3", true);
        //test("1.2.3-pre+asdf - 2.4.3-pre+asdf", "1.2.3-pre.2");
        //test("1.2.3-pre+asdf - 2.4.3-pre+asdf", "2.4.3-alpha");
        test("1.2.3+asdf - 2.4.3+asdf", "1.2.3");
        test("1.0.0", "1.0.0");
        test(">=*", "0.2.4");
        test("", "1.0.0");
        //test("*", "1.2.3", {});
        //test("*", "v1.2.3", { loose: 123 });
        //test(">=1.0.0", "1.0.0", /asdf/);
        //test(">=1.0.0", "1.0.1", { loose: null });
        //test(">=1.0.0", "1.1.0", { loose: 0 });
        //test(">1.0.0", "1.0.1", { loose: undefined });
        test(">1.0.0", "1.1.0");
        test("<=2.0.0", "2.0.0");
        test("<=2.0.0", "1.9999.9999");
        test("<=2.0.0", "0.2.9");
        test("<2.0.0", "1.9999.9999");
        test("<2.0.0", "0.2.9");
        test(">= 1.0.0", "1.0.0");
        test(">=  1.0.0", "1.0.1");
        test(">=   1.0.0", "1.1.0");
        test("> 1.0.0", "1.0.1");
        test(">  1.0.0", "1.1.0");
        test("<=   2.0.0", "2.0.0");
        test("<= 2.0.0", "1.9999.9999");
        test("<=  2.0.0", "0.2.9");
        test("<    2.0.0", "1.9999.9999");
        test("<\t2.0.0", "0.2.9");
        //test(">=0.1.97", "v0.1.97", true);
        test(">=0.1.97", "0.1.97");
        test("0.1.20 || 1.2.4", "1.2.4");
        test(">=0.2.3 || <0.0.1", "0.0.0");
        test(">=0.2.3 || <0.0.1", "0.2.3");
        test(">=0.2.3 || <0.0.1", "0.2.4");
        test("||", "1.3.4");
        test("2.x.x", "2.1.3");
        test("1.2.x", "1.2.3");
        test("1.2.x || 2.x", "2.1.3");
        test("1.2.x || 2.x", "1.2.3");
        test("x", "1.2.3");
        test("2.*.*", "2.1.3");
        test("1.2.*", "1.2.3");
        test("1.2.* || 2.*", "2.1.3");
        test("1.2.* || 2.*", "1.2.3");
        test("*", "1.2.3");
        test("2", "2.1.2");
        test("2.3", "2.3.1");
        test("~0.0.1", "0.0.1");
        test("~0.0.1", "0.0.2");
        test("~x", "0.0.9");
        // >=2.4.0 <2.5.0
        test("~2", "2.0.9");
        // >=2.4.0 <2.5.0
        test("~2.4", "2.4.0");
        // >=2.4.0 <2.5.0
        test("~2.4", "2.4.5");
        //~> not supported test("~>3.2.1", "3.2.2");
        // >=3.2.1 <3.3.0,
        test("~1", "1.2.3");
        // >=.0.0 <2.0.0
        //~> not supported test("~>1", "1.2.3");
        //~> not supported test("~> 1", "1.2.3");
        test("~1.0", "1.0.2");
        // >=.0.0 <1.1.0,
        test("~ 1.0", "1.0.2");
        test("~ 1.0.3", "1.0.12");
        //test("~ 1.0.3alpha", "1.0.12", { loose: true });
        test(">=1", "1.0.0");
        test(">= 1", "1.0.0");
        test("<1.2", "1.1.1");
        test("< 1.2", "1.1.1");
        test("~v0.5.4-pre", "0.5.5");
        test("~v0.5.4-pre", "0.5.4");
        test("=0.7.x", "0.7.2");
        test("<=0.7.x", "0.7.2");
        test(">=0.7.x", "0.7.2");
        test("<=0.7.x", "0.6.2");
        test("~1.2.1 >=1.2.3", "1.2.3");
        test("~1.2.1 =1.2.3", "1.2.3");
        test("~1.2.1 1.2.3", "1.2.3");
        test("~1.2.1 >=1.2.3 1.2.3", "1.2.3");
        test("~1.2.1 1.2.3 >=1.2.3", "1.2.3");
        test(">=1.2.1 1.2.3", "1.2.3");
        test("1.2.3 >=1.2.1", "1.2.3");
        test(">=1.2.3 >=1.2.1", "1.2.3");
        test(">=1.2.1 >=1.2.3", "1.2.3");
        test(">=1.2", "1.2.8");
        test("^1.2.3", "1.8.1");
        test("^0.1.2", "0.1.2");
        test("^0.1", "0.1.2");
        test("^0.0.1", "0.0.1");
        test("^1.2", "1.4.2");
        test("^1.2 ^1", "1.4.2");
        test("^1.2.3-alpha", "1.2.3-pre");
        test("^1.2.0-alpha", "1.2.0-pre");
        test("^0.0.1-alpha", "0.0.1-beta");
        test("^0.0.1-alpha", "0.0.1");
        test("^0.1.1-alpha", "0.1.1-beta");
        test("^x", "1.2.3");
        test("x - 1.0.0", "0.9.7");
        test("x - 1.x", "0.9.7");
        test("1.0.0 - x", "1.9.7");
        test("1.x - x", "1.9.7");
        test("<=7.x", "7.9.9");
        //test("2.x", "2.0.0-pre.0", { includePrerelease: true });
        //test("2.x", "2.1.0-pre.0", { includePrerelease: true });
        //test("1.1.x", "1.1.0-a", { includePrerelease: true });
        //test("1.1.x", "1.1.1-a", { includePrerelease: true });
        //test("*", "1.0.0-rc1", { includePrerelease: true });
        //test("^1.0.0-0", "1.0.1-rc1", { includePrerelease: true });
        //test("^1.0.0-rc2", "1.0.1-rc1", { includePrerelease: true });
        //test("^1.0.0", "1.0.1-rc1", { includePrerelease: true });
        //test("^1.0.0", "1.1.0-rc1", { includePrerelease: true });
        //test("1 - 2", "2.0.0-pre", { includePrerelease: true });
        //test("1 - 2", "1.0.0-pre", { includePrerelease: true });
        //test("1.0 - 2", "1.0.0-pre", { includePrerelease: true });

        //test("=0.7.x", "0.7.0-asdf", { includePrerelease: true });
        //test(">=0.7.x", "0.7.0-asdf", { includePrerelease: true });
        //test("<=0.7.x", "0.7.0-asdf", { includePrerelease: true });

        //test(">=1.0.0 <=1.1.0", "1.1.0-pre", { includePrerelease: true });
    }

    #[test]
    fn test_match_negative() {
        fn test(range: &str, version: &str) {
            let range = VersionRange::from_str(range).expect(range);
            let version = Version::from_str(version).expect(version);
            assert!(
                !range.matches(&version),
                "{} should not matches {}",
                range,
                version
            );
        }
        // test set are from node-semver
        // Copyright (c) Isaac Z. Schlueter and Contributors
        // Originally under The ISC License
        // https://github.com/npm/node-semver/blob/3a8a4309ae986c1967b3073ba88c9e69433d44cb/test/fixtures/range-exclude.js

        test("1.0.0 - 2.0.0", "2.2.3");
        test("1.2.3+asdf - 2.4.3+asdf", "1.2.3-pre.2");
        test("1.2.3+asdf - 2.4.3+asdf", "2.4.3-alpha");
        test("^1.2.3+build", "2.0.0");
        test("^1.2.3+build", "1.2.0");
        test("^1.2.3", "1.2.3-pre");
        test("^1.2", "1.2.0-pre");
        test(">1.2", "1.3.0-beta");
        test("<=1.2.3", "1.2.3-beta");
        test("^1.2.3", "1.2.3-beta");
        test("=0.7.x", "0.7.0-asdf");
        test(">=0.7.x", "0.7.0-asdf");
        test("<=0.7.x", "0.7.0-asdf");
        //test("1", "1.0.0beta", { loose: 420 });
        //test("<1", "1.0.0beta", true);
        //test("< 1", "1.0.0beta", true);
        test("1.0.0", "1.0.1");
        test(">=1.0.0", "0.0.0");
        test(">=1.0.0", "0.0.1");
        test(">=1.0.0", "0.1.0");
        test(">1.0.0", "0.0.1");
        test(">1.0.0", "0.1.0");
        test("<=2.0.0", "3.0.0");
        test("<=2.0.0", "2.9999.9999");
        test("<=2.0.0", "2.2.9");
        test("<2.0.0", "2.9999.9999");
        test("<2.0.0", "2.2.9");
        //test(">=0.1.97", "v0.1.93", true);
        test(">=0.1.97", "0.1.93");
        test("0.1.20 || 1.2.4", "1.2.3");
        test(">=0.2.3 || <0.0.1", "0.0.3");
        test(">=0.2.3 || <0.0.1", "0.2.2");
        //test("2.x.x", "1.1.3", { loose: NaN });
        test("2.x.x", "3.1.3");
        test("1.2.x", "1.3.3");
        test("1.2.x || 2.x", "3.1.3");
        test("1.2.x || 2.x", "1.1.3");
        test("2.*.*", "1.1.3");
        test("2.*.*", "3.1.3");
        test("1.2.*", "1.3.3");
        test("1.2.* || 2.*", "3.1.3");
        test("1.2.* || 2.*", "1.1.3");
        test("2", "1.1.2");
        test("2.3", "2.4.1");
        test("~0.0.1", "0.1.0-alpha");
        test("~0.0.1", "0.1.0");
        test("~2.4", "2.5.0");
        // >=2.4.0 <2.5.0
        test("~2.4", "2.3.9");
        //test("~>3.2.1", "3.3.2");
        // >=3.2.1 <3.3.0
        //test("~>3.2.1", "3.2.0");
        // >=3.2.1 <3.3.0
        test("~1", "0.2.3");
        // >=1.0.0 <2.0.0
        //test("~>1", "2.2.3");
        test("~1.0", "1.1.0");
        // >=1.0.0 <1.1.0
        test("<1", "1.0.0");
        test(">=1.2", "1.1.1");
        //test("1", "2.0.0beta", true);
        test("~v0.5.4-beta", "0.5.4-alpha");
        test("=0.7.x", "0.8.2");
        test(">=0.7.x", "0.6.2");
        test("<0.7.x", "0.7.2");
        test("<1.2.3", "1.2.3-beta");
        test("=1.2.3", "1.2.3-beta");
        test(">1.2", "1.2.8");
        test("^0.0.1", "0.0.2-alpha");
        test("^0.0.1", "0.0.2");
        test("^1.2.3", "2.0.0-alpha");
        test("^1.2.3", "1.2.2");
        test("^1.2", "1.1.9");
        //test("*", "v1.2.3-foo", true);

        // rust: parse logic is separated
        // invalid versions never satisfy, but shouldn't throw
        //test("*", "not a version");
        //test(">=2", "glorp");
        //test(">=2", false);
        //test("2.x", "3.0.0-pre.0", { includePrerelease: true });
        //test("^1.0.0", "1.0.0-rc1", { includePrerelease: true });
        //test("^1.0.0", "2.0.0-rc1", { includePrerelease: true });
        //test("^1.2.3-rc2", "2.0.0", { includePrerelease: true });
        test("^1.0.0", "2.0.0-rc1");
        //test("1 - 2", "3.0.0-pre", { includePrerelease: true });
        test("1 - 2", "2.0.0-pre");
        test("1 - 2", "1.0.0-pre");
        test("1.0 - 2", "1.0.0-pre");
        test("1.1.x", "1.0.0-a");
        test("1.1.x", "1.1.0-a");
        test("1.1.x", "1.2.0-a");
        //test("1.1.x", "1.2.0-a", { includePrerelease: true });
        //test("1.1.x", "1.0.0-a", { includePrerelease: true });
        test("1.x", "1.0.0-a");
        test("1.x", "1.1.0-a");
        test("1.x", "1.2.0-a");
        //test("1.x", "0.0.0-a", { includePrerelease: true });
        //test("1.x", "2.0.0-a", { includePrerelease: true });
        test(">=1.0.0 <1.1.0", "1.1.0");
        //test(">=1.0.0 <1.1.0", "1.1.0", { includePrerelease: true });
        test(">=1.0.0 <1.1.0", "1.1.0-pre");
        test(">=1.0.0 <1.1.0-pre", "1.1.0-pre");
    }
}
