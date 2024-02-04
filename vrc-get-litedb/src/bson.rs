use std::fmt::{Debug, Formatter};

/// ObjectId in Bson. Used for identifying documents in a collection.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    bytes: [u8; 12],
}

impl Debug for ObjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0u8; b"ObjectId(012345678901234567890123)".len()];
        buffer[0..b"ObjectId(".len()].copy_from_slice(b"ObjectId(");
        hex::encode_to_slice(self.bytes, &mut buffer[b"ObjectId(".len()..][..24]).unwrap();
        buffer[buffer.len() - 1] = b')';
        f.write_str(unsafe { std::str::from_utf8_unchecked(&buffer) })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DateTime(u64);

impl DateTime {
    pub fn from_millis_since_epoch(millis: u64) -> Self {
        Self(millis)
    }

    pub fn as_millis_since_epoch(&self) -> u64 {
        self.0
    }
}

impl Debug for DateTime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("DateTime({})", self.0))
    }
}
