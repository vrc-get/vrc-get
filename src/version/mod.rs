pub use range::DependencyRange;
pub use range::VersionRange;
pub use unity_version::UnityVersion;
pub use unity_version::ReleaseType;
use std::fmt::{Display, Formatter};
pub use version::Version;

macro_rules! from_str_impl {
    ($ty: ty) => {
        impl FromStr for $ty {
            type Err = ParseRangeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut buffer = ParsingBuf::new(s);
                let result = FromParsingBuf::parse(&mut buffer)?;
                if buffer.first().is_some() {
                    return Err(ParseRangeError::invalid_char(buffer.first_char()));
                }
                Ok(result)
            }
        }
    };
}

macro_rules! serialize_to_string {
    ($ty: ty) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(&std::string::ToString::to_string(self))
            }
        }
    };
}

macro_rules! deserialize_from_str {
    ($ty: ty, $name: literal) => {
        impl<'de> ::serde::de::Deserialize<'de> for $ty {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::de::Deserializer<'de>,
            {
                struct Visitor;
                impl<'de> ::serde::de::Visitor<'de> for Visitor {
                    type Value = $ty;

                    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                        formatter.write_str($name)
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: ::serde::de::Error,
                    {
                        std::str::FromStr::from_str(v).map_err(E::custom)
                    }
                }
                deserializer.deserialize_str(Visitor)
            }
        }
    };
}

mod range;
mod unity_version;
mod version;
mod identifier;

type Segment = u64;

const NOT_EXISTS: Segment = Segment::MAX;
const STAR: Segment = NOT_EXISTS - 1;
const UPPER_X: Segment = STAR - 1;
const LOWER_X: Segment = UPPER_X - 1;
const VERSION_SEGMENT_MAX: Segment = LOWER_X - 1;

trait FromParsingBuf: Sized {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseRangeError>;
}

struct ParsingBuf<'a> {
    buf: &'a str,
}

impl<'a> ParsingBuf<'a> {
    pub fn new(source: &'a str) -> ParsingBuf {
        Self { buf: source }
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn read(&mut self, ch: char) -> Result<(), ParseRangeError> {
        match self.buf.chars().next() {
            Some(c) if c == ch => {
                self.skip();
                Ok(())
            }
            Some(c) => Err(ParseRangeError::invalid_char(c)),
            None => Err(ParseRangeError::unexpected_end()),
        }
    }

    pub fn first(&self) -> Option<u8> {
        self.buf.as_bytes().first().copied()
    }

    pub fn first_char(&self) -> char {
        self.buf.chars().next().expect("invalid state")
    }

    pub fn skip(&mut self) -> &mut Self {
        if self.buf.len() != 0 {
            self.buf = &self.buf[1..];
        }
        self
    }

    pub fn get(&self, index: usize) -> Option<u8> {
        self.buf.as_bytes().get(index).copied()
    }

    pub fn skip_ws(&mut self) {
        self.buf = self.buf.trim_start();
    }

    pub fn take(&mut self, count: usize) -> &str {
        let (a, b) = self.buf.split_at(count);
        self.buf = b;
        a
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

impl std::error::Error for ParseRangeError {}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_cmp() {
        fn test(greater: &str, lesser: &str) {
            let greater = Version::from_str(greater).expect(greater);
            let lesser = Version::from_str(lesser).expect(lesser);
            assert!(greater > lesser, "{} > {}", greater, lesser);
        }
        // test set are from node-semver
        // Copyright (c) Isaac Z. Schlueter and Contributors
        // Originally under The ISC License
        // https://github.com/npm/node-semver/blob/3a8a4309ae986c1967b3073ba88c9e69433d44cb/test/fixtures/comparisons.js

        test("0.0.0", "0.0.0-foo");
        test("0.0.1", "0.0.0");
        test("1.0.0", "0.9.9");
        test("0.10.0", "0.9.0");
        //test("0.99.0", "0.10.0", {});
        //test("2.0.0", "1.2.3", { loose: false });
        //test("v0.0.0", "0.0.0-foo", true);
        //test("v0.0.1", "0.0.0", { loose: true });
        //test("v1.0.0", "0.9.9", true);
        //test("v0.10.0", "0.9.0", true);
        //test("v0.99.0", "0.10.0", true);
        //test("v2.0.0", "1.2.3", true);
        //test("0.0.0", "v0.0.0-foo", true);
        //test("0.0.1", "v0.0.0", true);
        //test("1.0.0", "v0.9.9", true);
        //test("0.10.0", "v0.9.0", true);
        //test("0.99.0", "v0.10.0", true);
        //test("2.0.0", "v1.2.3", true);
        test("1.2.3", "1.2.3-asdf");
        test("1.2.3", "1.2.3-4");
        test("1.2.3", "1.2.3-4-foo");
        test("1.2.3-5-foo", "1.2.3-5");
        test("1.2.3-5", "1.2.3-4");
        test("1.2.3-5-foo", "1.2.3-5-Foo");
        test("3.0.0", "2.7.2+asdf");
        test("1.2.3-a.10", "1.2.3-a.5");
        test("1.2.3-a.b", "1.2.3-a.5");
        test("1.2.3-a.b", "1.2.3-a");
        test("1.2.3-a.b.c.10.d.5", "1.2.3-a.b.c.5.d.100");
        test("1.2.3-r2", "1.2.3-r100");
        test("1.2.3-r100", "1.2.3-R2");
    }
}
