/// ObjectId in Bson. Used for identifying documents in a collection.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ObjectId {
    bytes: [u8; 12],
}
