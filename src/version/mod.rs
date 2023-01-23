pub use range::VersionRange;
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
mod version;

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

    pub fn slip_ws(&mut self) {
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
