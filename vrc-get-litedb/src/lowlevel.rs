//! This module contains low-level functions for interacting with the C# code

/// Rust representation of the System.Runtime.InteropServices.GCHandle type  
/// 
/// This is actually a wrapper type of [`isize`] but this struct will call `GCHandle.Free()` when dropped
#[repr(transparent)]
pub struct GcHandle(isize);

impl Drop for GcHandle {
    fn drop(&mut self) {
        extern "C" {
            fn vrc_get_litedb_lowlevel_free_gc_handle(handle: isize);
        }

        // SAFETY: the C# code is safe
        unsafe { vrc_get_litedb_lowlevel_free_gc_handle(self.0) }
    }
}

/// FFI safe byte slice which might be owned (`Boxed<[u8]>`) or a `str`
/// 
/// This struct doesn't free the memory when dropped
#[repr(C)]
pub struct ByteSlice {
    ptr: *mut u8,
    len: usize,
}

impl ByteSlice {
    fn from_byte_slice(slice: &[u8]) -> Self {
        Self {
            ptr: slice.as_ptr() as *mut _,
            len: slice.len(),
        }
    }

    fn from_boxed_slice(slice: Box<[u8]>) -> Self {
        let ptr = Box::into_raw(slice);
        let len = unsafe { (*ptr).len() };
        Self { ptr: ptr as *mut _, len }
    }

    /// SAFETY: the caller must ensure that the pointer is valid and the length is correct
    unsafe fn as_byte_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }

    /// SAFETY: the caller must ensure that the pointer is valid,
    /// the length is correct, the pointer is allocated with `Box`,
    /// and there are no other box instance for the same pointer.
    #[must_use = "call `drop(Box::as_boxed_byte_slice(slice))` if you intend to drop the `Box`"]
    unsafe fn as_boxed_byte_slice(&self) -> Box<[u8]> {
        let slice_ptr = std::ptr::slice_from_raw_parts_mut(self.ptr, self.len);
        Box::from_raw(slice_ptr)
    }
}

#[no_mangle]
extern "C" fn vrc_get_litedb_lowlevel_alloc_byte_slice(len: usize) -> *mut u8 {
    let slice = vec![0; len].into_boxed_slice();
    ByteSlice::from_boxed_slice(slice).ptr
}

#[no_mangle]
extern "C" fn test_returns_hello_rust() -> ByteSlice {
    ByteSlice::from_byte_slice(b"Hello, Rust!")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_returns_hello_rust() {
        extern "C" {
            fn test_call_returns_hello_rust() -> bool;
        }

        // test performed in C# side
        assert!(unsafe { test_call_returns_hello_rust() });
    }

    #[test]
    fn test_call_returns_hello_csharp() {
        extern "C" {
            fn test_returns_hello_csharp() -> ByteSlice;
        }
        
        unsafe {
            let slice = test_returns_hello_csharp();
            assert_eq!(slice.as_byte_slice(), b"Hello, C#!");
            drop(ByteSlice::as_boxed_byte_slice(&slice));
        }
    }
}
