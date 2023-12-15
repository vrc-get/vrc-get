use std::alloc::{alloc, handle_alloc_error, Layout};
use std::ptr;
use std::fmt::{Debug, Formatter};
use std::mem::{align_of, size_of};
use serde::Serializer;

/// backend for BuildMeta or Prerelease
/// 
/// Since most prerelease are less then 8 bytes like `beta.1`, `alpha.2`, etc, so,
/// We inline the first 8 bytes if short, otherwise, we use a pointer to inline heap.
/// 
/// This technic is inspired by [semver crate] by dtolnay which is also under MIT License.
/// 
/// [semver crate]: https://github.com/dtolnay/semver
// if uppermost bit is 1, it's pointer to heap shifted 1 bit to right.
// otherwise, it's inline data with zero terminated (up to 8 bytes)
// on heap, the first u16 is length of string, the rest is string data
#[repr(C, align(8))]
pub struct Identifier {
    data: u64
}

const U16_SIZE: usize = size_of::<u16>();
const U16_ALIGN: usize = align_of::<u16>();
const HEAP_ALIGN: usize = if U16_ALIGN >= 2 { U16_ALIGN } else { 2 };
const HEAP_FLAG: u64 = 1 << 63;

#[cfg(target_pointer_width = "16")]
compile_error!("vrc-get does not support 16-bit targets");

impl Identifier {
    pub const EMPTY: Identifier = Identifier { data: 0 };

    /// Creates new Identifier
    /// 
    /// SAFETY: the string must be valid ASCII string.
    /// if it contain non-ASCII bytes, it will undefined behaviour
    pub unsafe fn new_unchecked(string: &str) -> Self {
        let len = string.len();
        if len == 0 {
            Self::EMPTY
        } else if len <= 8 {
            // inline data
            let mut data_bytes = [0u8; 8];
            data_bytes[..len].copy_from_slice(string.as_bytes());
            Self {
                data: u64::from_ne_bytes(data_bytes)
            }
        } else if len < u16::MAX as usize {
            // heap data
            let data_size = len + U16_SIZE;
            let layout = unsafe { Layout::from_size_align_unchecked(data_size, HEAP_ALIGN) };
            let ptr = unsafe { alloc(layout) } as *const u16;
            if ptr.is_null() {
                handle_alloc_error(layout);
            }
            unsafe {
                *(ptr as *mut u16) = len as u16;
                let data_ptr = ptr.add(1) as *mut u8;
                ptr::copy_nonoverlapping(string.as_ptr(), data_ptr, len);
            }

            Self {
                data: unsafe { ptr_to_repr(ptr) }
            }
        } else {
            panic!("too long identifier")
        }
    }

    /// Returns true if this is empty identifier
    pub(crate) fn is_empty(&self) -> bool {
        self.data == 0
    }

    fn is_inline(&self) -> bool {
        self.data & HEAP_FLAG == 0
    }

    fn is_heap(&self) -> bool {
        self.data & HEAP_FLAG != 0
    }

    pub fn as_str(&self) -> &str {
        if self.is_empty() {
            ""
        } else if self.is_inline() {
            unsafe {
                let inline_len = self.len_inline();
                let head_ptr = &self.data as *const u64 as *const u8;
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(head_ptr, inline_len))
            }
        } else {
            let ptr = unsafe { repr_to_ptr(self.data) };
            let size = unsafe { *ptr };
            let data_ptr = unsafe { ptr.add(1) as *const u8 };
            unsafe {
                std::str::from_utf8_unchecked(std::slice::from_raw_parts(data_ptr, size as usize))
            }
        }
    }

    unsafe fn len_inline(&self) -> usize {
        let repr = self.data;

        #[cfg(target_endian = "little")]
            let zero_bits_on_string_end = repr.leading_zeros();
        #[cfg(target_endian = "big")]
            let zero_bits_on_string_end = repr.trailing_zeros();

        8 - zero_bits_on_string_end as usize / 8
    }
}

impl Clone for Identifier {
    fn clone(&self) -> Self {
        if !self.is_heap() {
            Self{
                data: self.data
            }
        } else {
            let ptr = unsafe { repr_to_ptr(self.data) };
            let size = unsafe { *ptr };
            let data_size = size as usize + U16_SIZE;
            let layout = unsafe { Layout::from_size_align_unchecked(data_size, HEAP_ALIGN) };
            let new_ptr = unsafe { alloc(layout) } as *mut u16;
            if new_ptr.is_null() {
                handle_alloc_error(layout);
            }
            unsafe {
                ptr::copy_nonoverlapping(ptr, new_ptr, data_size);
            }
            Self {
                data: unsafe { ptr_to_repr(new_ptr) }
            }
        }
    }
}

impl Drop for Identifier {
    fn drop(&mut self) {
        if self.is_heap() {
            let ptr = unsafe { repr_to_ptr(self.data) };
            let size = unsafe { *ptr };
            let layout = unsafe { Layout::from_size_align_unchecked(size as usize + U16_SIZE, HEAP_ALIGN) };
            unsafe { std::alloc::dealloc(ptr as *mut u8, layout) }
        }
    }
}

impl PartialEq for Identifier {
    fn eq(&self, rhs: &Self) -> bool {
        if !self.is_heap() {
            // Fast path (most common)
            self.data == rhs.data
        } else if !rhs.is_heap() {
            false
        } else {
            self.as_str() == rhs.as_str()
        }
    }
}

// the data is not mutable so it's safe to send and sync
unsafe impl Send for Identifier {}
unsafe impl Sync for Identifier {}

impl Debug for Identifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.serialize_str(self.as_str())
    }
}

unsafe fn repr_to_ptr(repr: u64) -> *const u16 {
    debug_assert!(repr & HEAP_FLAG != 0);

    (repr << 1) as *const u16
}

unsafe fn ptr_to_repr(ptr: *const u16) -> u64 {
    debug_assert!(!ptr.is_null());
    debug_assert!(ptr as u64 & 1 == 0);

    (ptr as u64 >> 1) | HEAP_FLAG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identifier_size_align() {
        assert_eq!(size_of::<Identifier>(), 8);
        assert_eq!(align_of::<Identifier>(), 8);
    }

    #[test]
    fn test_new() {
        unsafe {
            assert_eq!(Identifier::new_unchecked("beta.1").as_str(), "beta.1");
            assert_eq!(Identifier::new_unchecked("beta.1").data, u64::from_ne_bytes(*b"beta.1\0\0"));

            assert_eq!(Identifier::new_unchecked("alpha.2").as_str(), "alpha.2");
            assert_eq!(Identifier::new_unchecked("alpha.2").data, u64::from_ne_bytes(*b"alpha.2\0"));

            assert_eq!(Identifier::new_unchecked("").as_str(), "");
            assert_eq!(Identifier::new_unchecked("").data, 0);

            assert_eq!(Identifier::new_unchecked("long-representation").as_str(), "long-representation");
            assert_eq!(Identifier::new_unchecked("long-representation").data & HEAP_FLAG, HEAP_FLAG);
        }
    }

    #[test]
    fn test_eq() {
        macro_rules! test_eq {
            ($literal: literal) => {
                assert_eq!(Identifier::new_unchecked($literal), Identifier::new_unchecked($literal));
            };
        }
        unsafe {
            test_eq!("beta.1");
            test_eq!("alpha.2");
            test_eq!("");
            test_eq!("long-representation");
        }
    }
}
