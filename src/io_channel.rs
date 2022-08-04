///! TODO
use std::ops::Deref;
use std::{fmt, marker::PhantomData, ptr::NonNull};

use crate::libspdk::{
    spdk_io_channel,
    spdk_io_channel_get_io_device_name,
    spdk_io_channel_iter,
    spdk_io_channel_iter_get_channel,
    spdk_put_io_channel,
    spdk_rs_io_channel_get_ctx,
    spdk_thread_get_name,
};

/// Wrapper for SPDK `spdk_io_channel` structure.
///
/// # Generic Arguments
///
/// * `T`: user-defined channel data owned by this channel.
#[derive(Copy, Debug)]
pub struct IoChannel<T> {
    /// TODO
    inner: NonNull<spdk_io_channel>,

    /// TODO
    _cd: PhantomData<T>,
}

impl<ChannelData> IoChannel<ChannelData> {
    /// Returns a reference to the channel data instance that this I/O channel
    /// owns.
    pub fn channel_data(&self) -> &ChannelData {
        unsafe {
            &*(spdk_rs_io_channel_get_ctx(self.inner.as_ptr())
                as *mut ChannelData)
        }
    }

    /// Returns a mutable reference to the channel data instance that this I/O
    /// channel owns.
    pub fn channel_data_mut(&mut self) -> &mut ChannelData {
        unsafe {
            &mut *(spdk_rs_io_channel_get_ctx(self.inner.as_ptr())
                as *mut ChannelData)
        }
    }

    /// Returns the name of the I/O channel which is used to register the
    /// device. This can either be a string containing the pointer address,
    /// or an actual name.
    fn name(&self) -> &str {
        unsafe {
            std::ffi::CStr::from_ptr(spdk_io_channel_get_io_device_name(
                self.as_ptr(),
            ))
            .to_str()
            .unwrap()
        }
    }

    /// TODO
    fn thread_name(&self) -> &str {
        unsafe {
            std::ffi::CStr::from_ptr(spdk_thread_get_name(
                self.inner.as_ref().thread,
            ))
            .to_str()
            .unwrap()
        }
    }

    /// Makes a new `IoChannel` wrapper from a raw SPDK structure pointer.
    ///
    /// # Arguments
    ///
    /// * `ptr`: TODO
    pub(crate) fn from_ptr(ptr: *mut spdk_io_channel) -> Self {
        Self {
            inner: NonNull::new(ptr).unwrap(),
            _cd: Default::default(),
        }
    }

    /// TODO
    fn as_ptr(&self) -> *mut spdk_io_channel {
        self.inner.as_ptr()
    }

    /// TODO
    pub fn legacy_as_ptr(&self) -> *mut spdk_io_channel {
        self.as_ptr()
    }

    /// Makes a new `IoChannel` wrapper from a raw `spdk_io_channel_iter`
    /// pointer.
    ///
    /// # Arguments
    ///
    /// * `ptr`: TODO
    pub fn from_iter(ptr: *mut spdk_io_channel_iter) -> Self {
        let io_chan = unsafe { spdk_io_channel_iter_get_channel(ptr) };
        Self::from_ptr(io_chan)
    }
}

impl<ChannelData> Clone for IoChannel<ChannelData> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _cd: Default::default(),
        }
    }
}

/// RAII wrapper for SPDK I/O channel.
/// When this structure is dropped, the channel is put back.
pub struct IoChannelGuard<T> {
    chan: IoChannel<T>,
}

impl<T> Deref for IoChannelGuard<T> {
    type Target = IoChannel<T>;

    fn deref(&self) -> &Self::Target {
        &self.chan
    }
}

impl<T> Drop for IoChannelGuard<T> {
    fn drop(&mut self) {
        unsafe { spdk_put_io_channel(self.chan.as_ptr()) }
    }
}

impl<T> IoChannelGuard<T> {
    /// TODO
    pub(crate) fn from_ptr(ptr: *mut spdk_io_channel) -> Self {
        Self {
            chan: IoChannel::from_ptr(ptr),
        }
    }
}

impl<T> fmt::Debug for IoChannelGuard<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "io channel {:p} on thread {} to bdev {}",
            self.as_ptr(),
            self.thread_name(),
            self.name()
        )
    }
}
