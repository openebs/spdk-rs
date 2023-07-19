use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ops::{Index, IndexMut},
    os::raw::c_void,
    ptr::NonNull,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use crate::libspdk;

/// A newtype wrapper for SPDK's `iovec`.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct IoVec(libspdk::iovec);

impl Index<u64> for IoVec {
    type Output = u8;

    fn index(&self, index: u64) -> &Self::Output {
        &self.as_slice()[index as usize]
    }
}

impl IndexMut<u64> for IoVec {
    fn index_mut(&mut self, index: u64) -> &mut Self::Output {
        &mut self.as_mut_slice()[index as usize]
    }
}

impl IoVec {
    /// Creates a new `IoVec` instance.
    #[inline(always)]
    pub fn new(iov_base: *mut c_void, iov_len: u64) -> Self {
        Self(libspdk::iovec {
            iov_base,
            iov_len,
        })
    }

    /// Returns length of the `IoVec` buffer.
    #[inline(always)]
    pub fn len(&self) -> u64 {
        self.0.iov_len
    }

    /// Sets length of the `IoVec` buffer.
    #[inline(always)]
    pub unsafe fn set_len(&mut self, new_len: u64) -> u64 {
        let t = self.0.iov_len;
        self.0.iov_len = new_len;
        t
    }

    /// Returns if the underlying buffer point is initialized (not null).
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        !self.0.iov_base.is_null()
    }

    /// Returns if the length of the `IoVec` buffer is empty.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `u8` slice representation of `IoVec`.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        assert!(self.is_initialized());
        unsafe {
            from_raw_parts(self.as_ptr() as *const _, self.len() as usize)
        }
    }

    /// Returns mutable `u8` slice representation of `IoVec`.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        assert!(self.is_initialized());
        unsafe {
            from_raw_parts_mut(self.as_mut_ptr() as *mut _, self.len() as usize)
        }
    }

    /// TODO
    #[inline(always)]
    pub fn as_ptr(&self) -> *const c_void {
        self.0.iov_base
    }

    /// TODO
    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut c_void {
        self.0.iov_base
    }

    /// Fills the buffer with the given value.
    pub fn fill(&mut self, val: u8) {
        unsafe {
            std::ptr::write_bytes(
                self.0.iov_base as *mut u8,
                val,
                self.0.iov_len as usize,
            )
        }
    }

    /// Compares two buffer and returns the index of the first mismatching
    /// byte.
    pub fn compare(a: &Self, b: &Self) -> Option<u64> {
        let len_a = a.len();
        let len_b = b.len();

        if len_a < len_b {
            return Some(len_a);
        }

        if len_a > len_b {
            return Some(len_b);
        }

        for i in 0 .. len_a {
            if a[i] != b[i] {
                return Some(i);
            }
        }

        None
    }
}

/// Trait to cast an array-like contain of `IoVec` to the underlying SPDK
/// `iovec` array pointer.
pub trait AsIoVecPtr {
    #[inline(always)]
    fn as_io_vec_ptr(&self) -> *const libspdk::iovec;

    #[inline(always)]
    fn as_io_vec_mut_ptr(&mut self) -> *mut libspdk::iovec;
}

impl AsIoVecPtr for [IoVec] {
    #[inline(always)]
    fn as_io_vec_ptr(&self) -> *const libspdk::iovec {
        self.as_ptr() as *const _
    }

    #[inline(always)]
    fn as_io_vec_mut_ptr(&mut self) -> *mut libspdk::iovec {
        self.as_mut_ptr() as *mut _
    }
}

/// Trait to cast an array-like container to an array of `IoVec`.
pub trait AsIoVecs {
    /// Casts an array-like object to a slice of `IoVec`.
    #[inline(always)]
    fn as_io_vecs(&self) -> &[IoVec];

    /// Casts a mutable array-like object to a mutable slice of `IoVec`.
    #[inline(always)]
    fn as_io_vecs_mut(&mut self) -> &mut [IoVec];
}
