use futures::channel::oneshot;
///! TODO
use std::{marker::PhantomData, os::raw::c_void, ptr::NonNull};

use crate::{
    ffihelper::{errno_error, ErrnoResult, IntoCString},
    io_channel::IoChannelGuard,
    libspdk::{
        bdev_lock_lba_range,
        bdev_unlock_lba_range,
        spdk_bdev,
        spdk_bdev_close,
        spdk_bdev_desc,
        spdk_bdev_desc_get_bdev,
        spdk_bdev_event_type,
        spdk_bdev_get_io_channel,
        spdk_bdev_open_ext,
        SPDK_BDEV_EVENT_MEDIA_MANAGEMENT,
        SPDK_BDEV_EVENT_REMOVE,
        SPDK_BDEV_EVENT_RESIZE,
    },
    Bdev,
    BdevOps,
};

/// Wrapper for `spdk_bdev_desc`.
///
/// # Notes
///
/// Multiple descriptors to the same Bdev are allowed. A Bdev can be claimed for
/// an exclusive write access. Any existing descriptors that are open before the
/// bdev has been claimed will remain as is. Typically, the target, exporting
/// the bdev will claim the device. In the case of the nexus, we do not claim
/// the children for exclusive access to allow for the rebuild to happen across
/// multiple cores.
///
/// # Generic Arguments
///
/// * `BdevData`: TODO
#[derive(Debug)]
pub struct BdevDesc<BdevData>
where
    BdevData: BdevOps,
{
    /// TODO
    inner: *mut spdk_bdev_desc,
    /// TODO
    _data: PhantomData<BdevData>,
}

// TODO: is `BdevDesc` really a Sync type?
unsafe impl<T: BdevOps> Sync for BdevDesc<T> {}
unsafe impl<T: BdevOps> Send for BdevDesc<T> {}

impl<BdevData> BdevDesc<BdevData>
where
    BdevData: BdevOps,
{
    /// TODO
    ///
    /// # Arguments
    ///
    /// * `bdev_name`: TODO
    /// * `rw`: TODO
    /// * `event_cb`: TODO
    pub fn open(
        bdev_name: &str,
        rw: bool,
        event_cb: fn(BdevEvent, Bdev<BdevData>),
    ) -> ErrnoResult<Self> {
        let mut desc: *mut spdk_bdev_desc = std::ptr::null_mut();

        // let ctx = Box::new(BdevEventContext::<BdevData> {
        //     event_cb: Box::new(event_cb),
        // });

        let rc = unsafe {
            spdk_bdev_open_ext(
                bdev_name.into_cstring().as_ptr(),
                rw,
                Some(inner_bdev_event_cb::<BdevData>),
                // Box::into_raw(ctx) as *mut c_void,
                event_cb as *mut c_void,
                &mut desc,
            )
        };

        if rc != 0 {
            errno_error::<Self>(rc)
        } else {
            assert_eq!(desc.is_null(), false);
            Ok(Self::from_ptr(desc))
        }
    }

    /// TODO
    pub fn close(&mut self) {
        assert!(!self.inner.is_null());

        unsafe {
            // Close the desc.
            spdk_bdev_close(self.as_ptr());
            self.inner = std::ptr::null_mut();
        }
    }

    /// Returns a Bdev associated with this descriptor.
    /// A descriptor cannot exist without a Bdev.
    pub fn bdev(&self) -> Bdev<BdevData> {
        let b = unsafe { spdk_bdev_desc_get_bdev(self.as_ptr()) };
        Bdev::from_inner_ptr(b)
    }

    /// Returns a channel to the underlying Bdev.
    pub fn io_channel(&self) -> Option<IoChannelGuard<BdevData::ChannelData>> {
        let ch = unsafe { spdk_bdev_get_io_channel(self.as_ptr()) };
        if ch.is_null() {
            // TODO: error message
            error!(
                "Failed to get IO channel for {}, probably low on memory!",
                self.bdev().name(),
            );
            None
        } else {
            Some(IoChannelGuard::from_ptr(ch))
        }
    }

    /// Gains exclusive access over a block range, and returns
    /// a lock object that must be used to unlock the range.
    pub async fn lock_lba_range(
        &self,
        range: LbaRange,
    ) -> Result<LbaRangeLock<BdevData>, nix::errno::Errno> {
        let (s, r) = oneshot::channel::<i32>();

        let ctx = Box::new(LockContext {
            offset: range.offset,
            len: range.len,
            ch: self.io_channel().unwrap(),
            sender: Some(s),
        });

        unsafe {
            let rc = bdev_lock_lba_range(
                self.as_ptr(),
                ctx.ch.legacy_as_ptr(),
                ctx.offset,
                ctx.len,
                Some(LockContext::<BdevData>::lock_completion_cb),
                ctx.as_ref() as *const _ as *mut c_void,
            );
            if rc != 0 {
                return Err(nix::errno::from_i32(rc));
            }
        }

        // Wait for the lock to complete
        let rc = r.await.unwrap();
        if rc != 0 {
            return Err(nix::errno::from_i32(rc));
        }

        Ok(LbaRangeLock {
            ctx,
        })
    }

    /// Releases exclusive access over a block range.
    pub async fn unlock_lba_range(
        &self,
        mut lock: LbaRangeLock<BdevData>,
    ) -> Result<(), nix::errno::Errno> {
        let (s, r) = oneshot::channel::<i32>();
        lock.ctx.sender = Some(s);

        unsafe {
            let rc = bdev_unlock_lba_range(
                self.as_ptr(),
                lock.ctx.ch.legacy_as_ptr(),
                lock.ctx.offset,
                lock.ctx.len,
                Some(LockContext::<BdevData>::lock_completion_cb),
                lock.ctx.as_ref() as *const _ as *mut c_void,
            );
            if rc != 0 {
                return Err(nix::errno::from_i32(rc));
            }
        }

        // Wait for the unlock to complete
        let rc = r.await.unwrap();
        if rc != 0 {
            return Err(nix::errno::from_i32(rc));
        }

        Ok(())
    }

    /// Returns a pointer to the underlying `spdk_bdev_desc` structure.
    pub(crate) fn as_ptr(&self) -> *mut spdk_bdev_desc {
        self.inner
    }

    /// TODO
    pub fn legacy_as_ptr(&self) -> *mut spdk_bdev_desc {
        self.as_ptr()
    }

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `ptr`: TODO
    pub(crate) fn from_ptr(ptr: *mut spdk_bdev_desc) -> Self {
        assert!(!ptr.is_null());

        Self {
            inner: ptr,
            _data: Default::default(),
        }
    }

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `ptr`: TODO
    pub fn legacy_from_ptr(ptr: *mut spdk_bdev_desc) -> Self {
        Self::from_ptr(ptr)
    }
}

impl<BdevData> Clone for BdevDesc<BdevData>
where
    BdevData: BdevOps,
{
    fn clone(&self) -> Self {
        assert!(!self.inner.is_null());

        Self {
            inner: self.inner,
            _data: Default::default(),
        }
    }
}

/// TODO
pub enum BdevEvent {
    /// TODO
    Remove,
    /// TODO
    Resize,
    /// TODO
    MediaManagement,
}

impl From<spdk_bdev_event_type> for BdevEvent {
    fn from(t: spdk_bdev_event_type) -> Self {
        match t {
            SPDK_BDEV_EVENT_REMOVE => BdevEvent::Remove,
            SPDK_BDEV_EVENT_RESIZE => BdevEvent::Resize,
            SPDK_BDEV_EVENT_MEDIA_MANAGEMENT => BdevEvent::MediaManagement,
            _ => panic!("Bad Bdev event type: {}", t),
        }
    }
}

impl From<BdevEvent> for spdk_bdev_event_type {
    fn from(t: BdevEvent) -> Self {
        match t {
            BdevEvent::Remove => SPDK_BDEV_EVENT_REMOVE,
            BdevEvent::Resize => SPDK_BDEV_EVENT_RESIZE,
            BdevEvent::MediaManagement => SPDK_BDEV_EVENT_MEDIA_MANAGEMENT,
        }
    }
}

/// TODO
///
/// # Generic Arguments
///
/// * `BdevData`: TODO
///
/// # Arguments
///
/// * `event`: TODO
/// * `bdev`: TODO
/// * `ctx`: TODO
unsafe extern "C" fn inner_bdev_event_cb<BdevData>(
    event: spdk_bdev_event_type,
    bdev: *mut spdk_bdev,
    ctx: *mut c_void,
) where
    BdevData: BdevOps,
{
    let ctx = std::mem::transmute::<_, fn(BdevEvent, Bdev<BdevData>)>(ctx);
    (ctx)(event.into(), Bdev::<BdevData>::from_inner_ptr(bdev));
}

/// TODO
pub struct LbaRange {
    pub offset: u64,
    pub len: u64,
}

impl LbaRange {
    /// Creates a new LbaRange.
    pub fn new(offset: u64, len: u64) -> LbaRange {
        LbaRange {
            offset,
            len,
        }
    }
}

/// TODO
struct LockContext<T: BdevOps> {
    offset: u64,
    len: u64,
    ch: IoChannelGuard<T::ChannelData>,
    sender: Option<oneshot::Sender<i32>>,
}

impl<T: BdevOps> LockContext<T> {
    unsafe extern "C" fn lock_completion_cb(
        ctx: *mut ::std::os::raw::c_void,
        status: ::std::os::raw::c_int,
    ) {
        let ctx = &mut *(ctx as *mut Self);
        let s = ctx.sender.take().unwrap();

        // Send a notification that the operation has completed.
        if let Err(e) = s.send(status) {
            panic!("Failed to send SPDK completion with error {}.", e);
        }
    }
}

pub struct LbaRangeLock<T: BdevOps> {
    ctx: Box<LockContext<T>>,
}
