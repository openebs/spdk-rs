//! The buffers written to the bdev must be allocated by the provided allocation
//! methods. These buffers are allocated from mem pools and huge pages and allow
//! for DMA transfers in the case of, for example, NVMe devices.

use std::{
    ffi::c_void,
    ops::{Deref, DerefMut},
    slice::{from_raw_parts, from_raw_parts_mut},
};

use snafu::Snafu;

use crate::{
    libspdk::{
        spdk_dma_free,
        spdk_zmalloc,
        SPDK_ENV_LCORE_ID_ANY,
        SPDK_MALLOC_DMA,
    },
    AsIoVecs,
    IoVec,
};

#[derive(Debug, Snafu, Clone)]
pub enum DmaError {
    #[snafu(display("Failed to allocate DMA buffer"))]
    Alloc {},
}

/// `DmaBuf` that is allocated from the memory pool.
/// It has the same representation as `IoVec` and SPDK's `iovec`, and can be
/// used in place of them. `DmaBuf` owns its buffer and deallocates on drop,
/// while `IoVec` does not do that as it is just a Rust-style interface for
/// `iovec`.
#[derive(Debug)]
#[repr(transparent)]
pub struct DmaBuf(IoVec);

// TODO: is `DmaBuf` really a Send type?
unsafe impl Send for DmaBuf {}

impl Drop for DmaBuf {
    fn drop(&mut self) {
        unsafe { spdk_dma_free(self.as_mut_ptr() as *mut c_void) }
    }
}

impl Deref for DmaBuf {
    type Target = IoVec;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DmaBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl DmaBuf {
    /// Allocates a buffer suitable for IO (wired and backed by huge page
    /// memory).
    ///
    /// # Arguments
    ///
    /// * `size`: TODO
    /// * `alignment`: TODO
    pub fn new(size: u64, alignment: u64) -> Result<Self, DmaError> {
        let buf = unsafe {
            spdk_zmalloc(
                size,
                alignment,
                std::ptr::null_mut(),
                SPDK_ENV_LCORE_ID_ANY as i32,
                SPDK_MALLOC_DMA,
            )
        };

        if buf.is_null() {
            Err(DmaError::Alloc {})
        } else {
            Ok(Self(IoVec::new(buf, size)))
        }
    }

    /// Returns an `IoVec` instance pointing to this buffer.
    #[inline(always)]
    pub fn to_io_vec(&self) -> IoVec {
        self.0
    }
}

impl AsIoVecs for [DmaBuf] {
    #[inline(always)]
    fn as_io_vecs(&self) -> &[IoVec] {
        unsafe { from_raw_parts(self.as_ptr() as *const IoVec, self.len()) }
    }

    #[inline(always)]
    fn as_io_vecs_mut(&mut self) -> &mut [IoVec] {
        unsafe { from_raw_parts_mut(self.as_ptr() as *mut IoVec, self.len()) }
    }
}

impl AsIoVecs for Vec<DmaBuf> {
    #[inline(always)]
    fn as_io_vecs(&self) -> &[IoVec] {
        unsafe { from_raw_parts(self.as_ptr() as *const IoVec, self.len()) }
    }

    #[inline(always)]
    fn as_io_vecs_mut(&mut self) -> &mut [IoVec] {
        unsafe { from_raw_parts_mut(self.as_ptr() as *mut IoVec, self.len()) }
    }
}
