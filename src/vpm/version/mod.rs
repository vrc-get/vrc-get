pub use range::VersionRange;

mod range;
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
