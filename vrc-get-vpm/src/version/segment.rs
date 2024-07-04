use std::fmt::{Debug, Formatter};
use std::str::FromStr;

pub(super) enum SegmentParseErr {
    #[allow(dead_code)]
    IntError(std::num::ParseIntError),
    TooBig,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub(super) struct Segment(pub(super) u64);

impl Segment {
    pub const NOT_EXISTS: Segment = Segment(u64::MAX);
    pub const STAR: Segment = Segment(u64::MAX - 1);
    pub const UPPER_X: Segment = Segment(u64::MAX - 2);
    pub const LOWER_X: Segment = Segment(u64::MAX - 3);
    pub const MAX: Segment = Segment(u64::MAX / 2);

    pub const ZERO: Segment = Segment(0);

    pub(super) fn as_number(self) -> Option<u64> {
        if self.0 <= Self::MAX.0 {
            Some(self.0)
        } else {
            None
        }
    }

    pub fn new(value: u64) -> Option<Segment> {
        if value <= Self::MAX.0 {
            Some(Segment(value))
        } else {
            None
        }
    }
}

impl Debug for Segment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            Segment::NOT_EXISTS => write!(f, "Segment(NOT_EXISTS)"),
            Segment::STAR => write!(f, "Segment(STAR)"),
            Segment::UPPER_X => write!(f, "Segment(UPPER_X)"),
            Segment::LOWER_X => write!(f, "Segment(LOWER_X)"),
            Segment(v) => write!(f, "Segment({v})"),
        }
    }
}

impl FromStr for Segment {
    type Err = SegmentParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x" => Ok(Segment::LOWER_X),
            "X" => Ok(Segment::UPPER_X),
            "*" => Ok(Segment::STAR),
            _ => {
                let value = s.parse().map_err(SegmentParseErr::IntError)?;
                if value > Self::MAX.0 {
                    return Err(SegmentParseErr::TooBig);
                }
                Ok(Segment(value))
            }
        }
    }
}
