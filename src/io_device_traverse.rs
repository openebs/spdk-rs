use async_trait::async_trait;
use futures::channel::oneshot;
use std::{marker::PhantomData, os::raw::c_void};

use crate::{
    libspdk::{
        spdk_for_each_channel,
        spdk_for_each_channel_continue,
        spdk_io_channel_iter,
        spdk_io_channel_iter_get_ctx,
    },
    IoChannel,
    IoDevice,
};

/// TODO
#[derive(Debug)]
pub enum ChannelTraverseStatus {
    /// TODO
    Ok,
    /// TODO
    Cancel,
}

impl From<i32> for ChannelTraverseStatus {
    fn from(i: i32) -> Self {
        match i {
            0 => Self::Ok,
            _ => Self::Cancel,
        }
    }
}

impl From<ChannelTraverseStatus> for i32 {
    fn from(s: ChannelTraverseStatus) -> Self {
        match s {
            ChannelTraverseStatus::Ok => 0,
            ChannelTraverseStatus::Cancel => 1,
        }
    }
}

/// TODO
///
/// # Generic Arguments
///
/// * `'a`: TODO
/// * `'b`: TODO
/// * `'c`: TODO
/// * `'ChannelData`: TODO
/// * `'Ctx`: TODO
struct TraverseCtx<'a, 'b, 'c, ChannelData, Ctx> {
    /// TODO
    channel_cb: Box<
        dyn FnMut(&mut ChannelData, &mut Ctx) -> ChannelTraverseStatus + 'a,
    >,
    /// TODO
    done_cb: Box<dyn FnMut(ChannelTraverseStatus, Ctx) + 'b>,
    /// TODO
    ctx: Ctx,
    /// TODO
    _cd: PhantomData<ChannelData>,
    /// TODO
    _c: PhantomData<&'c ()>,
}

impl<'a, 'b, 'c, ChannelData, Ctx> TraverseCtx<'a, 'b, 'c, ChannelData, Ctx> {
    /// TODO
    ///
    /// # Arguments
    ///
    /// * `channel_cb`: TODO
    /// * `done_cb`: TODO
    /// * `caller_ctx`: TODO
    fn new(
        channel_cb: impl FnMut(&mut ChannelData, &mut Ctx) -> ChannelTraverseStatus
            + 'a,
        done_cb: impl FnMut(ChannelTraverseStatus, Ctx) + 'b,
        caller_ctx: Ctx,
    ) -> Self {
        Self {
            channel_cb: Box::new(channel_cb),
            done_cb: Box::new(done_cb),
            ctx: caller_ctx,
            _cd: Default::default(),
            _c: Default::default(),
        }
    }

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `i`: TODO
    #[inline]
    fn from_iter(i: *mut spdk_io_channel_iter) -> &'c mut Self {
        unsafe { &mut *(spdk_io_channel_iter_get_ctx(i) as *mut Self) }
    }
}

/// TODO
#[async_trait(?Send)]
pub trait IoDeviceChannelTraverse: IoDevice {
    /// Iterates over all I/O channels associated with this I/O device.
    ///
    /// # Arguments
    ///
    /// * `channel_cb`: TODO
    /// * `done_cb`: TODO
    /// * `context`: TODO
    fn traverse_io_channels<'a, 'b, Ctx>(
        &self,
        context: Ctx,
        channel_cb: impl FnMut(
                &mut <Self as IoDevice>::ChannelData,
                &mut Ctx,
            ) -> ChannelTraverseStatus
            + 'a,
        done_cb: impl FnMut(ChannelTraverseStatus, Ctx) + 'b,
    ) {
        let ctx = Box::new(TraverseCtx::new(channel_cb, done_cb, context));

        // Start I/O channel iteration via SPDK.
        unsafe {
            spdk_for_each_channel(
                self.get_io_device_id(),
                Some(inner_traverse_channel::<Self::ChannelData, Ctx>),
                Box::into_raw(ctx) as *mut c_void,
                Some(inner_traverse_channel_done::<Self::ChannelData, Ctx>),
            );
        }
    }

    /// Asynchrnously iterates over all I/O channels associated with this I/O
    /// device.
    async fn traverse_io_channels_async<T, F>(&self, data: T, func: F)
    where
        T: Send,
        F: FnMut(&mut <Self as IoDevice>::ChannelData, &T) -> (),
    {
        let (sender, recv) = oneshot::channel::<()>();

        let ctx = TraverseAsyncCtx::<Self, T, F> {
            sender,
            data,
            func,
            _d: Default::default(),
        };

        self.traverse_io_channels(
            ctx,
            TraverseAsyncCtx::channel_cb,
            TraverseAsyncCtx::channel_done,
        );

        recv.await
            .expect("for_each_io_channel(): sender already dropped: {err}");
    }
}

/// Low-level per-channel visitor to be invoked by SPDK I/O channel
/// enumeration logic.
///
/// # Arguments
///
/// * `i`: TODO
extern "C" fn inner_traverse_channel<ChannelData, Ctx>(
    i: *mut spdk_io_channel_iter,
) {
    let ctx = TraverseCtx::<ChannelData, Ctx>::from_iter(i);
    let mut chan = IoChannel::<ChannelData>::from_iter(i);

    let rc = (ctx.channel_cb)(chan.channel_data_mut(), &mut ctx.ctx);

    unsafe {
        spdk_for_each_channel_continue(i, rc.into());
    }
}

/// Low-level completion callback for SPDK I/O channel enumeration logic.
extern "C" fn inner_traverse_channel_done<ChannelData, Ctx>(
    i: *mut spdk_io_channel_iter,
    status: i32,
) {
    // Reconstruct the context box to let all the resources be properly
    // dropped.
    let ctx = TraverseCtx::<ChannelData, Ctx>::from_iter(i);
    let mut ctx = unsafe { Box::from_raw(ctx) };
    (ctx.done_cb)(status.into(), ctx.ctx);
}

/// TODO
struct TraverseAsyncCtx<D, T, F>
where
    D: IoDevice,
    T: Send,
    F: FnMut(&mut D::ChannelData, &T) -> (),
{
    sender: oneshot::Sender<()>,
    data: T,
    func: F,
    _d: PhantomData<D>,
}

/// TODO
impl<D, T, F> TraverseAsyncCtx<D, T, F>
where
    D: IoDevice,
    T: Send,
    F: FnMut(&mut D::ChannelData, &T) -> (),
{
    fn channel_cb(
        channel: &mut D::ChannelData,
        ctx: &mut Self,
    ) -> ChannelTraverseStatus {
        (ctx.func)(channel, &ctx.data);
        ChannelTraverseStatus::Ok
    }

    fn channel_done(_status: ChannelTraverseStatus, ctx: Self) {
        ctx.sender.send(()).expect("Receiver disappeared");
    }
}
