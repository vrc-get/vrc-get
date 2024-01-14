//! This module contains low-level functions for interacting with the C# code

use std::alloc::Layout;
use std::num::NonZeroIsize;

/// Rust representation of the System.Runtime.InteropServices.GCHandle type  
/// 
/// This is actually a wrapper type of [`isize`] but this struct will call `GCHandle.Free()` when dropped
#[repr(transparent)]
pub struct GcHandle(NonZeroIsize);

impl Drop for GcHandle {
    fn drop(&mut self) {
        extern "C" {
            fn vrc_get_litedb_lowlevel_free_gc_handle(handle: NonZeroIsize);
        }

        // SAFETY: the C# code is safe
        unsafe { vrc_get_litedb_lowlevel_free_gc_handle(self.0) }
    }
}

/// FFI safe byte slice which might be owned (`Boxed<[u8]>`) or a `str`
/// 
/// This struct doesn't free the memory when dropped
#[repr(C)]
pub struct FFISlice<T = u8> {
    ptr: *mut T,
    len: usize,
}

impl<T> FFISlice<T> {
    pub fn from_byte_slice(slice: &[T]) -> Self {
        Self {
            ptr: slice.as_ptr() as *mut _,
            len: slice.len(),
        }
    }

    pub fn from_boxed_slice(slice: Box<[T]>) -> Self {
        let ptr = Box::into_raw(slice);
        let len = unsafe { (*ptr).len() };
        Self { ptr: ptr as *mut _, len }
    }

    /// SAFETY: the caller must ensure that the pointer is valid and the length is correct
    pub unsafe fn as_byte_slice(&self) -> &[T] {
        std::slice::from_raw_parts(self.ptr, self.len)
    }

    /// SAFETY: the caller must ensure that the pointer is valid,
    /// the length is correct, the pointer is allocated with `Box`,
    /// and there are no other box instance for the same pointer.
    #[must_use = "call `drop(Box::as_boxed_byte_slice(slice))` if you intend to drop the `Box`"]
    pub unsafe fn as_boxed_byte_slice(&self) -> Box<[T]> {
        let slice_ptr = std::ptr::slice_from_raw_parts_mut(self.ptr, self.len);
        Box::from_raw(slice_ptr)
    }
}

#[no_mangle]
unsafe extern "C" fn vrc_get_litedb_lowlevel_alloc(size: usize, align: usize) -> *mut u8 {
    let layout = Layout::from_size_align_unchecked(size, align);
    std::alloc::alloc(layout)
}

#[no_mangle]
extern "C" fn test_returns_hello_rust() -> FFISlice {
    FFISlice::from_byte_slice(b"Hello, Rust!")
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
            fn test_returns_hello_csharp() -> FFISlice;
        }
        
        unsafe {
            let slice = test_returns_hello_csharp();
            assert_eq!(slice.as_byte_slice(), b"Hello, C#!");
            drop(FFISlice::as_boxed_byte_slice(&slice));
        }
    }

    #[test]
    fn struct_size_offset_test() {
        use std::mem::size_of;
        macro_rules! offset_of {
            ($ty:ty, $field:tt) => {
                {
                    let uninitialized = std::mem::MaybeUninit::<$ty>::uninit();
                    let ptr = uninitialized.as_ptr();
                    let field = unsafe { &(*ptr).$field as *const _ };
                    let offset = field as usize - ptr as usize;
                    offset
                }
            }
        }

        let ptr_size = size_of::<*mut u8>();

        assert_eq!(size_of::<GcHandle>(), ptr_size);
        assert_eq!(offset_of!(GcHandle, 0), 0);

        assert_eq!(size_of::<FFISlice>(), 2 * ptr_size);
        assert_eq!(offset_of!(FFISlice, ptr), 0);
        assert_eq!(offset_of!(FFISlice, len), ptr_size);
    }

    #[test]
    fn struct_size_offset_test_cs() {
        extern "C" {
            fn test_struct_size_offset_test_cs() -> bool;
        }

        let successful = unsafe { test_struct_size_offset_test_cs() };

        assert!(successful);
    }

    #[test]
    fn throw_csharp_test() {
        extern "C" {
            fn throws_exception_cs();
        }

        unsafe { throws_exception_cs(); }
    }
}
