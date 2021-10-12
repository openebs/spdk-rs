///! TODO
use crate::{BdevIo, IoChannel, IoDevice, IoType, JsonWriteContext};
use std::pin::Pin;

/// TODO
pub trait BdevOps {
    /// Data type of Bdev I/O channel data.
    type ChannelData;

    /// TODO
    type BdevData: BdevOps;

    /// TODO
    type IoDev: IoDevice;

    /// TODO
    fn destruct(self: Pin<&mut Self>);

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `chan`: TODO
    /// * `bio`: TODO
    fn submit_request(
        &self,
        chan: IoChannel<Self::ChannelData>,
        bio: BdevIo<Self::BdevData>,
    );

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
