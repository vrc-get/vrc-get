use std::fmt::{Debug, Formatter};

/// ObjectId in Bson. Used for identifying documents in a collection.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ObjectId {
    bytes: [u8; 12],
}

impl Debug for ObjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut buffer = [0u8; b"ObjectId(0123456789012345678901234)".len()];
        buffer[0..b"ObjectId(".len()].copy_from_slice(b"ObjectId(");
        hex::encode_to_slice(self.bytes, &mut buffer[b"ObjectId(".len()..][..24]).unwrap();
        buffer[buffer.len() - 1] = b')';
        f.write_str(unsafe { std::str::from_utf8_unchecked(&buffer) })
    }
}
