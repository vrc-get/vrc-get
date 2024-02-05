use once_cell::sync::Lazy;
use rand::Rng;
use std::fmt::{Debug, Formatter};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// ObjectId in Bson. Used for identifying documents in a collection.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId {
    bytes: [u8; 12],
}

impl ObjectId {
    pub fn new() -> ObjectId {
        let timestamp = ObjectId::gen_timestamp();
        let process_id = ObjectId::gen_process_id();
        let counter = ObjectId::gen_count();

        let mut buf: [u8; 12] = [0; 12];

        buf[0..][..4].clone_from_slice(&timestamp[..]);
        buf[4..][..5].clone_from_slice(&process_id[..]);
        buf[9..][..3].clone_from_slice(&counter[..]);

        ObjectId::from_bytes(&buf)
    }

    fn gen_timestamp() -> [u8; 4] {
        let timestamp: u32 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        timestamp.to_be_bytes()
    }

    fn gen_process_id() -> [u8; 5] {
        static BUF: Lazy<[u8; 5]> = Lazy::new(rand::random);

        *BUF
    }

    fn gen_count() -> [u8; 3] {
        static OID_COUNTER: Lazy<AtomicU32> =
            Lazy::new(|| AtomicU32::new(rand::thread_rng().gen_range(0..=0xFF_FF_FF)));
        let u_counter = OID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let u_int = u_counter % 0x1_00_00_00;
        let buf = u_int.to_be_bytes();
        let buf_u24: [u8; 3] = [buf[1], buf[2], buf[3]];
        buf_u24
    }
}

impl ObjectId {
    pub fn from_bytes(bytes: &[u8; 12]) -> Self {
        Self { bytes: *bytes }
    }
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
    pub fn now() -> DateTime {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).unwrap();
        Self(duration.as_millis() as u64)
    }
}

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
