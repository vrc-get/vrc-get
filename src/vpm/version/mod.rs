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
