use crate::version::*;
use std::fmt::{Display, Formatter, Write};
use std::str::FromStr;

// TODO: TEST

#[derive(::serde::Serialize, ::serde::Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct DependencyRange(VersionRange);

impl DependencyRange {
    /// create from one version
    pub fn version(version: Version) -> DependencyRange {
        Self(VersionRange {
            comparators: vec![ComparatorSet(vec![Comparator::Star(PartialVersion::from(
                version,
            ))])],
        })
    }

    pub fn from_version_range(version_range: VersionRange) -> DependencyRange {
        let range = DependencyRange(version_range);
        if let Some(full_version) = range.as_single_version() {
            // If the version is like '1.0.0', it will mean '>= 1.0.0' with DependencyRange,
            // However, we should treat '1.0.0' as '=1.0.0' so replace with that
            Self(VersionRange {
                comparators: vec![ComparatorSet(vec![Comparator::Exact(
                    PartialVersion::from(full_version),
                )])],
            })
        } else {
            range
        }
    }

    pub fn as_single_version(&self) -> Option<Version> {
        let [ComparatorSet(the_set)] = self.0.comparators.as_slice() else {
            return None;
        };

        let [Comparator::Star(star)] = &the_set[..] else {
            return None;
        };

        let full = star.to_full()?;

        Some(full)
    }

    pub fn matches(&self, version: &Version) -> bool {
        if let Some(single) = self.as_single_version() {
            &single <= version
        } else {
            self.0.match_pre(version, PrereleaseAcceptance::Allow)
        }
    }

    pub fn as_range(&self) -> VersionRange {
        self.as_single_version()
            .map(VersionRange::same_or_later)
            .unwrap_or_else(|| self.0.clone())
    }
}

impl Display for DependencyRange {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VersionRange {
    comparators: Vec<ComparatorSet>,
}

#[derive(Clone, Copy)]
pub enum PrereleaseAcceptance {
    Deny,
    Allow,
    Minimum,
}

impl PrereleaseAcceptance {
    pub(crate) fn allow_or_minimum(allow: bool) -> PrereleaseAcceptance {
        if allow {
            PrereleaseAcceptance::Allow
        } else {
            PrereleaseAcceptance::Minimum
        }
    }
}

impl VersionRange {
    pub fn same_or_later(version: Version) -> Self {
        Self {
            comparators: vec![ComparatorSet(vec![Comparator::GreaterThanOrEqual(
                PartialVersion::from(version),
            )])],
        }
    }

    pub fn specific(version: Version) -> VersionRange {
        Self {
            comparators: vec![ComparatorSet(vec![Comparator::Exact(
                PartialVersion::from(version),
            )])],
        }
    }

    pub fn contains_pre(&self) -> bool {
        self.comparators.iter().any(ComparatorSet::contains_pre)
    }

    pub fn matches(&self, version: &Version) -> bool {
        self.match_pre(version, PrereleaseAcceptance::Minimum)
    }

    pub fn match_pre(&self, version: &Version, allow_prerelease: PrereleaseAcceptance) -> bool {
        self.comparators
            .iter()
            .any(|x| x.matches(version, allow_prerelease))
    }

    pub fn intersect(&self, other: &VersionRange) -> VersionRange {
        VersionRange {
            // TODO: remove contradictory
            comparators: (self.comparators.iter())
                .flat_map(|self_cmp| {
                    other.comparators.iter().map(|other_cmp| {
                        ComparatorSet(
                            self_cmp
                                .0
                                .iter()
                                .chain(other_cmp.0.iter())
                                .cloned()
                                .collect(),
                        )
                    })
                })
                .collect(),
        }
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
        for next in iter {
            f.write_str(" || ")?;
            Display::fmt(next, f)?
        }
        Ok(())
    }
}

impl FromStr for VersionRange {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            comparators: s
                .split("||")
                .map(FromStr::from_str)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ComparatorSet(Vec<Comparator>);

impl Display for ComparatorSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.iter();
        match iter.next() {
            None => return Ok(()),
            Some(next) => Display::fmt(next, f)?,
        }
        for next in iter {
            f.write_str(" ")?;
            Display::fmt(next, f)?
        }
        Ok(())
    }
}

from_str_impl!(ComparatorSet);

impl FromParsingBuf for ComparatorSet {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
        let mut result = Vec::<Comparator>::new();

        while !buffer.is_empty() {
            result.push(Comparator::parse(buffer)?);
            buffer.skip_ws();
        }

        Ok(Self(result))
    }
}

impl ComparatorSet {
    fn matches(&self, version: &Version, allow_prerelease: PrereleaseAcceptance) -> bool {
        self.0.iter().all(|x| x.matches(version, allow_prerelease))
    }

    fn contains_pre(&self) -> bool {
        self.0.iter().any(Comparator::contains_pre)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
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
            Comparator::Hyphen(a, b) => write!(f, "{a} - {b}"),
            Comparator::Star(v) => Display::fmt(v, f),
        }
    }
}

impl Comparator {
    fn matches(&self, version: &Version, allow_prerelease: PrereleaseAcceptance) -> bool {
        if !self.matches_internal(version) {
            return false;
        }

        if version.is_stable() {
            return true;
        }

        // for pre-release, depends on allow_prerelease
        match allow_prerelease {
            PrereleaseAcceptance::Deny => false,
            PrereleaseAcceptance::Allow => true,
            PrereleaseAcceptance::Minimum => {
                let in_versions: &[&PartialVersion] = match self {
                    Self::Tilde(c) => &[c],
                    Self::Caret(c) => &[c],
                    Self::Exact(c) => &[c],
                    Self::GreaterThan(c) => &[c],
                    Self::GreaterThanOrEqual(c) => &[c],
                    Self::LessThan(c) => &[c],
                    Self::LessThanOrEqual(c) => &[c],
                    Self::Star(c) => &[c],
                    Self::Hyphen(c, d) => &[c, d],
                };

                for version in in_versions {
                    let version = version.to_zeros();
                    if version.is_pre() && version.base_version() == version.base_version() {
                        return true;
                    }
                }
                false
            }
        }
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
                if v.major().is_none() {
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
            Comparator::Star(v) | Comparator::Exact(v) => match v.to_full_or_next() {
                (full, true) => full.cmp(version).is_eq(),
                (next, false) => &v.to_zeros_with_pre() <= version && version < &next,
            },
            Comparator::GreaterThan(v) => greater_than(version, v),
            Comparator::GreaterThanOrEqual(v) => greater_than_or_equal(version, v),
            Comparator::LessThan(v) => less_than(version, v),
            Comparator::LessThanOrEqual(v) => less_than_or_equal(version, v),
            Comparator::Hyphen(lower, upper) => {
                greater_than_or_equal(version, lower) && less_than_or_equal(version, upper)
            }
        };

        fn greater_than(version: &Version, v: &PartialVersion) -> bool {
            match v.to_full_or_next() {
                (full, true) => version > &full,
                (next, false) => version >= &next,
            }
        }
        fn greater_than_or_equal(version: &Version, v: &PartialVersion) -> bool {
            match v.to_full() {
                Some(v) => version >= &v,
                None => version >= &v.to_zeros_with_pre(),
            }
        }
        fn less_than(version: &Version, v: &PartialVersion) -> bool {
            match v.to_full() {
                Some(v) => version < &v,
                None => version < &v.to_zeros_with_pre(),
            }
        }

        fn less_than_or_equal(version: &Version, v: &PartialVersion) -> bool {
            match v.to_full_or_next() {
                (full, true) => version <= &full,
                (next, false) => version < &next,
            }
        }
    }

    fn contains_pre(&self) -> bool {
        match self {
            Comparator::Tilde(v)
            | Comparator::Caret(v)
            | Comparator::Exact(v)
            | Comparator::GreaterThan(v)
            | Comparator::GreaterThanOrEqual(v)
            | Comparator::LessThan(v)
            | Comparator::LessThanOrEqual(v)
            | Comparator::Star(v) => !v.pre.is_empty(),
            Comparator::Hyphen(a, b) => !a.pre.is_empty() || !b.pre.is_empty(),
        }
    }
}

impl FromParsingBuf for Comparator {
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
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
            None => Err(ParseVersionError::unexpected_end()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct PartialVersion {
    major: Segment,
    minor: Segment,
    patch: Segment,
    pre: Prerelease,
    build: BuildMetadata,
}

impl Display for PartialVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn write_segment(f: &mut Formatter<'_>, value: Segment, prefix: &str) -> std::fmt::Result {
            if value != Segment::NOT_EXISTS {
                f.write_str(prefix)?;
                match value {
                    Segment::STAR => f.write_char('*')?,
                    Segment::UPPER_X => f.write_char('X')?,
                    Segment::LOWER_X => f.write_char('x')?,
                    _ => Display::fmt(&value.0, f)?,
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

    /// Returns (version, is_full)
    fn to_full_or_next(&self) -> (Version, bool) {
        if let Some(major) = self.major() {
            if let Some(minor) = self.minor() {
                if let Some(patch) = self.patch() {
                    (
                        Version {
                            major,
                            minor,
                            patch,
                            pre: self.pre.clone(),
                            build: self.build.clone(),
                        },
                        true,
                    )
                } else {
                    (
                        Version::new_pre(major, minor + 1, 0, Prerelease::new("0").unwrap()),
                        false,
                    )
                }
            } else {
                (
                    Version::new_pre(major + 1, 0, 0, Prerelease::new("0").unwrap()),
                    false,
                )
            }
        } else {
            (
                Version::new_pre(u64::MAX, u64::MAX, u64::MAX, Prerelease::new("0").unwrap()),
                false,
            )
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

    fn to_zeros_with_pre(&self) -> Version {
        Version {
            major: self.major_or(0),
            minor: self.minor_or(0),
            patch: self.patch_or(0),
            pre: if self.pre.is_empty() {
                Prerelease::new("0").unwrap()
            } else {
                self.pre.clone()
            },
            build: self.build.clone(),
        }
    }

    fn major(&self) -> Option<u64> {
        self.major.as_number()
    }

    fn major_or(&self, default: u64) -> u64 {
        self.major.as_number().unwrap_or(default)
    }

    fn minor(&self) -> Option<u64> {
        self.minor.as_number()
    }

    fn minor_or(&self, default: u64) -> u64 {
        self.minor.as_number().unwrap_or(default)
    }

    fn patch(&self) -> Option<u64> {
        self.patch.as_number()
    }

    fn patch_or(&self, default: u64) -> u64 {
        self.patch.as_number().unwrap_or(default)
    }
}

impl FromParsingBuf for PartialVersion {
    fn parse(bytes: &mut ParsingBuf) -> Result<Self, ParseVersionError> {
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
            Segment::NOT_EXISTS
        };
        let (patch, pre, build) = if let Some(b'.') = bytes.first() {
            bytes.skip();
            let patch = parse_segment(bytes)?;

            let prerelease = if let Some(b'-') = bytes.first() {
                if bytes.get(1).is_none() || bytes.get(1) == Some(b'.') {
                    // '-' can be start of prerelease
                    Prerelease::new(Self::parse_id(bytes, false)?).unwrap()
                } else {
                    bytes.skip();
                    Prerelease::new(Self::parse_id(bytes, false)?).unwrap()
                }
            } else if bytes.first().map(Self::is_id_start).unwrap_or(false) {
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
            (Segment::NOT_EXISTS, Prerelease::EMPTY, BuildMetadata::EMPTY)
        };

        return Ok(PartialVersion {
            major,
            minor,
            patch,
            pre,
            build,
        });

        fn parse_segment(bytes: &mut ParsingBuf) -> Result<Segment, ParseVersionError> {
            match bytes.first() {
                Some(b'x') => {
                    bytes.skip();
                    Ok(Segment::LOWER_X)
                }
                Some(b'X') => {
                    bytes.skip();
                    Ok(Segment::UPPER_X)
                }
                Some(b'*') => {
                    bytes.skip();
                    Ok(Segment::STAR)
                }
                Some(b'1'..=b'9') => {
                    let mut i = 1;
                    while let Some(b'0'..=b'9') = bytes.get(i) {
                        i += 1;
                    }
                    let str = bytes.take(i);
                    let value = Segment::from_str(str).map_err(|_| ParseVersionError::too_big())?;
                    Ok(value)
                }
                Some(b'0') => {
                    bytes.skip();
                    // if 0\d, 0 is invalid char
                    if let Some(b'0'..=b'9') = bytes.first() {
                        return Err(ParseVersionError::invalid());
                    }
                    Ok(Segment::ZERO)
                }
                Some(_) => Err(ParseVersionError::invalid()),
                None => Err(ParseVersionError::invalid()),
            }
        }
    }
}

impl PartialVersion {
    fn is_id_start(b: u8) -> bool {
        matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-')
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

        if bytes.buf.is_empty() {
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
            major: Segment::new(value.major).unwrap(),
            minor: Segment::new(value.minor).unwrap(),
            patch: Segment::new(value.patch).unwrap(),
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
            assert!(range.matches(&version), "{range} matches {version}");
        }
        fn test_pre(range: &str, version: &str) {
            let range = VersionRange::from_str(range).expect(range);
            let version = Version::from_str(version).expect(version);
            assert!(
                range.match_pre(&version, PrereleaseAcceptance::Allow),
                "{range} matches {version}"
            );
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
        test("1.2.3-pre+asdf - 2.4.3-pre+asdf", "1.2.3-pre.2");
        test("1.2.3-pre+asdf - 2.4.3-pre+asdf", "2.4.3-alpha");
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
        test_pre("2.x", "2.0.0-pre.0");
        test_pre("2.x", "2.1.0-pre.0");
        test_pre("1.1.x", "1.1.0-a");
        test_pre("1.1.x", "1.1.1-a");
        test_pre("*", "1.0.0-rc1");
        test_pre("^1.0.0-0", "1.0.1-rc1");
        test_pre("^1.0.0-rc2", "1.0.1-rc1");
        test_pre("^1.0.0", "1.0.1-rc1");
        test_pre("^1.0.0", "1.1.0-rc1");
        test_pre("1 - 2", "2.0.0-pre");
        test_pre("1 - 2", "1.0.0-pre");
        test_pre("1.0 - 2", "1.0.0-pre");

        test_pre("=0.7.x", "0.7.0-asdf");
        test_pre(">=0.7.x", "0.7.0-asdf");
        test_pre("<=0.7.x", "0.7.0-asdf");

        test_pre(">=1.0.0 <=1.1.0", "1.1.0-pre");

        // additional tests by anatawa12
        test_pre("1.0.x - 2", "1.0.0-pre");
        // hyphen-less prerelease in range
        test("~3.5.0beta", "3.5.0");
        test("~3.5.0-", "3.5.0");
    }

    #[test]
    fn test_match_negative() {
        fn test(range: &str, version: &str) {
            let range = VersionRange::from_str(range).expect(range);
            let version = Version::from_str(version).expect(version);
            assert!(
                !range.matches(&version),
                "{range} should not matches {version}"
            );
        }
        fn test_pre(range: &str, version: &str) {
            let range = VersionRange::from_str(range).expect(range);
            let version = Version::from_str(version).expect(version);
            assert!(
                !range.match_pre(&version, PrereleaseAcceptance::Allow),
                "{range} should not matches {version}"
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
        test_pre("2.x", "3.0.0-pre.0");
        test_pre("^1.0.0", "1.0.0-rc1");
        test_pre("^1.0.0", "2.0.0-rc1");
        test_pre("^1.2.3-rc2", "2.0.0");
        test("^1.0.0", "2.0.0-rc1");
        test_pre("1 - 2", "3.0.0-pre");
        test("1 - 2", "2.0.0-pre");
        test("1 - 2", "1.0.0-pre");
        test("1.0 - 2", "1.0.0-pre");
        test("1.1.x", "1.0.0-a");
        test("1.1.x", "1.1.0-a");
        test("1.1.x", "1.2.0-a");
        test_pre("1.1.x", "1.2.0-a");
        test_pre("1.1.x", "1.0.0-a");
        test("1.x", "1.0.0-a");
        test("1.x", "1.1.0-a");
        test("1.x", "1.2.0-a");
        test_pre("1.x", "0.0.0-a");
        test_pre("1.x", "2.0.0-a");
        test(">=1.0.0 <1.1.0", "1.1.0");
        test_pre(">=1.0.0 <1.1.0", "1.1.0");
        test(">=1.0.0 <1.1.0", "1.1.0-pre");
        test(">=1.0.0 <1.1.0-pre", "1.1.0-pre");
    }
}
