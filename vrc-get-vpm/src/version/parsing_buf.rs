use std::fmt::{Display, Formatter};

pub(super) trait FromParsingBuf: Sized {
    fn parse(buffer: &mut ParsingBuf) -> Result<Self, ParseVersionError>;
}

pub(super) struct ParsingBuf<'a> {
    pub(super) buf: &'a str,
}

impl<'a> ParsingBuf<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { buf: source }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn read(&mut self, ch: char) -> Result<(), ParseVersionError> {
        match self.buf.chars().next() {
            Some(c) if c == ch => {
                self.skip();
                Ok(())
            }
            Some(_) => Err(ParseVersionError::invalid()),
            None => Err(ParseVersionError::invalid()),
        }
    }

    pub fn first(&self) -> Option<u8> {
        self.buf.as_bytes().first().copied()
    }

    pub fn skip(&mut self) -> &mut Self {
        if !self.buf.is_empty() {
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
pub struct ParseVersionError {
    inner: Inner,
}

impl Display for ParseVersionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.inner {
            Inner::VersionSegmentTooBig => f.write_str("version segment too big"),
            Inner::UnexpectedEnd => f.write_str("unexpected end"),
            Inner::Invalid => write!(f, "invalid"),
        }
    }
}

impl std::error::Error for ParseVersionError {}

#[derive(Debug)]
enum Inner {
    VersionSegmentTooBig,
    UnexpectedEnd,
    Invalid,
}

impl ParseVersionError {
    pub(super) fn too_big() -> Self {
        Self {
            inner: Inner::VersionSegmentTooBig,
        }
    }
    pub(super) fn invalid() -> Self {
        Self {
            inner: Inner::Invalid,
        }
    }
    pub(super) fn unexpected_end() -> ParseVersionError {
        Self {
            inner: Inner::UnexpectedEnd,
        }
    }
}
