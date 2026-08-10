//! TODO
use crate::{BdevIo, IoChannel, IoDevice, IoType, JsonWriteContext};
use std::{os::raw::c_void, pin::Pin};

/// Describes whether a bdev destructor has completed in-line or if we need to
/// await for the I/O device unregister callback.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum BdevDestruct {
    /// Destruction completed synchronously.
    Sync,
    /// Destruction will complete through an I/O-device unregister callback.
    Async,
}

/// Deferred completion of an asynchronous bdev destruction.
pub struct BdevDestructCompletion {
    pub(crate) bdev: *mut crate::libspdk::spdk_bdev,
    pub(crate) context: *mut c_void,
    pub(crate) drop_context: unsafe fn(*mut c_void),
}

/// TODO
pub trait BdevOps {
    /// Data type of Bdev I/O channel data.
    type ChannelData;

    /// TODO
    type BdevData: BdevOps;

    /// TODO
    type IoDev: IoDevice;

    /// TODO
    fn destruct(self: Pin<&mut Self>) -> BdevDestruct;

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `chan`: TODO
    /// * `bio`: TODO
    fn submit_request(&self, chan: IoChannel<Self::ChannelData>, bio: BdevIo<Self::BdevData>);

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `io_type`: TODO
    fn io_type_supported(&self, io_type: IoType) -> bool;

    /// TODO
    fn get_io_device(&self) -> &Self::IoDev;

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `w`: TODO
    fn dump_info_json(&self, _w: JsonWriteContext) {}
}
