///! Asynchronous methods of `Bdev<>` wrapper.
use std::os::raw::c_void;

use futures::channel::{oneshot, oneshot::Canceled};

use crate::{
    error::{SpdkError::BdevUnregisterFailed, SpdkResult},
    ffihelper::{
        cb_arg,
        done_errno_cb,
        errno_error,
        errno_result_from_i32,
        ErrnoResult,
    },
    libspdk::{
        bdev_reset_device_stat,
        spdk_bdev,
        spdk_bdev_get_device_stat,
        spdk_bdev_io_stat,
        spdk_bdev_unregister,
        SPDK_BDEV_RESET_STAT_ALL,
    },
    Bdev,
    BdevOps,
};

/// TODO
pub type BdevStats = spdk_bdev_io_stat;

/// TODO
pub struct BdevAsyncCallContext {
    /// TODO
    pub bdev: *mut spdk_bdev,
    /// TODO
    pub cb: Option<unsafe extern "C" fn(*mut c_void, i32)>,
    /// TODO
    pub arg: *mut c_void,
}

impl<BdevData> Bdev<BdevData>
where
    BdevData: BdevOps,
{
    /// TODO
    ///
    /// # Arguments
    ///
    /// * `f`: TODO
    pub async fn call_errno_fn_async(
        &mut self,
        f: impl Fn(BdevAsyncCallContext),
    ) -> Result<ErrnoResult<()>, Canceled> {
        let (s, r) = oneshot::channel::<ErrnoResult<()>>();
        let ctx = BdevAsyncCallContext {
            bdev: self.as_inner_ptr(),
            cb: Some(done_errno_cb),
            arg: cb_arg(s),
        };
        f(ctx);
        r.await
    }

    /// TODO
    pub async fn unregister_bdev_async(&mut self) -> SpdkResult<()> {
        let name = self.name().to_string();
        let (s, r) = oneshot::channel::<bool>();

        unsafe {
            spdk_bdev_unregister(
                self.as_inner_ptr(),
                Some(inner_unregister_callback),
                Box::into_raw(Box::new(s)) as *mut _,
            );
        }

        if r.await.unwrap() {
            Ok(())
        } else {
            Err(BdevUnregisterFailed {
                name,
            })
        }
    }

    /// Get bdev IOStats or errno value in case of an error.
    pub async fn stats_async(&self) -> ErrnoResult<BdevStats> {
        let mut stat: spdk_bdev_io_stat = unsafe { std::mem::zeroed() };
        let (s, r) = oneshot::channel::<i32>();

        // This will iterate over I/O channels and call async callback when
        // done.
        unsafe {
            spdk_bdev_get_device_stat(
                self.as_inner_ptr(),
                &mut stat as *mut _,
                Some(inner_stats_callback),
                cb_arg(s),
            );
        }

        let errno = r.await.expect("Cancellation is not supported");
        errno_result_from_i32(stat, errno)
    }

    /// This function resets all stat counters for a given Bdev.
    /// Returns Errno in case of an error.
    pub async fn stats_reset_async(&self) -> ErrnoResult<()> {
        let (s, r) = oneshot::channel::<i32>();
        // This will iterate over I/O channels to reset IOStats and call async
        // callback when done.
        unsafe {
            bdev_reset_device_stat(
                self.as_inner_ptr(),
                SPDK_BDEV_RESET_STAT_ALL,
                Some(inner_stats_reset_callback),
                cb_arg(s),
            );
        }
        let errno = r.await.expect("Cancellation is not supported");
        if errno == 0 {
            return Ok(());
        }
        errno_error(errno)
    }
}

/// TODO
/// TODO: used to synchronize the destroy call
///
/// # Arguments
///
/// * `arg`: TODO
/// * `rc`: TODO
///
/// # Safety
///
/// TODO
unsafe extern "C" fn inner_unregister_callback(arg: *mut c_void, rc: i32) {
    let s = Box::from_raw(arg as *mut oneshot::Sender<bool>);
    let _ = match rc {
        0 => s.send(true),
        _ => s.send(false),
    };
}

///
/// # Arguments
/// Callback function for spdk_bdev_get_device_stat function.
/// Will be called by SPDK on the completion of the call.
/// * `bdev`: bdev pointer. Will not do anything on it in callback.
/// * `stat`: stat struct which we pass to the SPDK fn.
/// * `arg`: Sender handle of channel to send errno.
/// * `errno`: Errno resulted in the function call.
///
/// # Safety
unsafe extern "C" fn inner_stats_callback(
    _bdev: *mut spdk_bdev,
    _stat: *mut spdk_bdev_io_stat,
    arg: *mut c_void,
    errno: i32,
) {
    let s = Box::from_raw(arg as *mut oneshot::Sender<i32>);
    s.send(errno)
        .expect("`inner_stats_callback()` receiver is gone");
}

///
/// # Arguments
/// Callback function for bdev_reset_device_stat function.
/// Will be called by SPDK on the completion of the call.
/// * `bdev`: bdev pointer. Will not do anything on it in callback.
/// * `arg`: Its a Sender handle of channel to send errno.
/// * `errno`: Errno resulted in the function call.
///
/// # Safety
unsafe extern "C" fn inner_stats_reset_callback(
    _bdev: *mut spdk_bdev,
    arg: *mut c_void,
    errno: i32,
) {
    let s = Box::from_raw(arg as *mut oneshot::Sender<i32>);
    s.send(errno)
        .expect("`inner_stats_reset_callback()` receiver handler is gone");
}
