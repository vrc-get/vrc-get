use std::path::Path;

#[repr(C)]
struct ObjectId {
    bytes: [u8; 12],
}

#[repr(transparent)]
struct ProjectType(u32);

struct Project {
    path: Box<Path>,
    unity_version: Box<str>,
    created_at: u64,
    updated_at: u64,
    type_: ProjectType,
    id: ObjectId,
    favorite: bool,
}

struct FFIBoxedBytes {
    ptr: *mut u8,
    len: usize,
}

#[repr(C)]
struct ProjectFFI {
    path: FFIBoxedBytes,
    unity_version: FFIBoxedBytes,
    created_at: u64,
    updated_at: u64,
    type_: ProjectType,
    id: ObjectId,
    favorite: bool,
}

#[link(name = "vrc_get_libdb_cs_alloc_bytes")]
extern "C" fn alloc_bytes(len: usize) -> FFIBoxedBytes {
    let mut bytes = Vec::with_capacity(len);
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    FFIBoxedBytes { ptr, len }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use std::mem::size_of;
    use std::ptr::addr_of;
    use super::*;

    macro_rules! offset {
        ($t: ty > $field: ident) => {
            unsafe {
                let obj = std::mem::MaybeUninit::<Project>::uninit();
                let ptr = obj.as_ptr();
                let offset = addr_of!((*ptr).$field) as usize;
                std::mem::forget(obj);
                offset - ptr as usize
            }
        };
    }

    #[test]
    fn rust_offsets() {
        assert_eq!(offset!(Project > path), 0);
        assert_eq!(offset!(Project > unity_version), size_of::<usize>() * 2);
        assert_eq!(offset!(Project > created_at), size_of::<usize>() * 4);
        assert_eq!(offset!(Project > updated_at), size_of::<usize>() * 4 + 8);
        assert_eq!(offset!(Project > type_), size_of::<usize>() * 4 + 16);
        assert_eq!(offset!(Project > id), size_of::<usize>() * 4 + 20);
        assert_eq!(offset!(Project > favorite), size_of::<usize>() * 4 + 32);
    }

    #[test]
    fn ffi_offsets() {
        assert_eq!(offset!(ProjectFFI > path), 0);
        assert_eq!(offset!(ProjectFFI > unity_version), size_of::<usize>() * 2);
        assert_eq!(offset!(ProjectFFI > created_at), size_of::<usize>() * 4);
        assert_eq!(offset!(ProjectFFI > updated_at), size_of::<usize>() * 4 + 8);
        assert_eq!(offset!(ProjectFFI > type_), size_of::<usize>() * 4 + 16);
        assert_eq!(offset!(ProjectFFI > id), size_of::<usize>() * 4 + 20);
        assert_eq!(offset!(ProjectFFI > favorite), size_of::<usize>() * 4 + 32);
    }
}
