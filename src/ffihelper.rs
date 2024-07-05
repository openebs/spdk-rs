use std::{
    error::Error,
    ffi::{CStr, CString},
    os::{
        raw,
        raw::{c_char, c_void},
    },
    ptr::{copy_nonoverlapping, NonNull},
};

use futures::channel::{
    oneshot,
    oneshot::{Receiver, Sender},
};
use nix::errno::Errno;

/// TODO
pub fn pair<T>() -> (Sender<T>, Receiver<T>) {
    oneshot::channel::<T>()
}

/// TODO
pub trait AsStr {
    /// TODO
    fn as_str(&self) -> &str;
}

impl AsStr for *const c_char {
    fn as_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(*self).to_str().unwrap_or_else(|_| {
                warn!("invalid UTF8 data");
                Default::default()
            })
        }
    }
}

impl AsStr for *mut c_char {
    fn as_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(*self).to_str().unwrap_or_else(|_| {
                warn!("invalid UTF8 data");
                Default::default()
            })
        }
    }
}

impl AsStr for [c_char] {
    fn as_str(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.as_ptr()).to_str().unwrap_or_else(|_| {
                warn!("invalid UTF8 data");
                Default::default()
            })
        }
    }
}

/// TODO
pub trait IntoCString {
    /// TODO
    fn into_cstring(self) -> CString;
}

impl IntoCString for String {
    fn into_cstring(self) -> CString {
        CString::new(self).unwrap()
    }
}

impl IntoCString for &str {
    fn into_cstring(self) -> CString {
        CString::new(self).unwrap()
    }
}

/// Copies a Rust string into a character buffer, always terminating with
/// the null byte. If the string to be copied (including the terminating null
/// byte) is longer than the destination, it is truncated to fit the
/// destination.
pub fn copy_str_with_null(src: &str, dst: &mut [c_char]) {
    let csrc = src.into_cstring();
    copy_cstr_with_null(&csrc, dst);
}

/// Copies a CString into a character buffer, always terminating with
/// the null byte. If the string to be copied (including the terminating null
/// byte) is longer than the destination, it is truncated to fit the
/// destination.
pub fn copy_cstr_with_null(src: &CStr, dst: &mut [c_char]) {
    copy_cstr_to_buf_with_null(&src, dst.as_mut_ptr(), dst.len());
}

/// Copies a CString into a character buffer, always terminating with
/// the null byte. If the string to be copied (including the terminating null
/// byte) is longer than the destination, it is truncated to fit the
/// destination.
pub fn copy_cstr_to_buf_with_null(
    src: &CStr,
    dst: *mut c_char,
    dst_size: usize,
) {
    let bytes = src.to_bytes();
    let count = std::cmp::min(bytes.len(), dst_size - 1);
    unsafe {
        copy_nonoverlapping(bytes.as_ptr() as *const c_char, dst, count);
        let dst = dst.add(count);
        *dst = 0 as c_char;
    }
}

/// Result having Errno error.
pub type ErrnoResult<T, E = Errno> = Result<T, E>;

/// Constructs a callback argument for spdk async function.
/// The argument is a oneshot sender channel for result of the operation.
/// The pointer returned by this function is a raw pointer to
/// a heap-allocated object, and it must be consumed by `done_cb`,
/// `done_errno_cb`, or dropped explicitly by `drop_cb_arg`.
pub fn cb_arg<T>(sender: oneshot::Sender<T>) -> *mut c_void {
    Box::into_raw(Box::new(sender)) as *const _ as *mut c_void
}

/// Drops a callback argument contructed by `cb_arg`.
/// This is needed when the callback is known to be not called.
pub fn drop_cb_arg<T>(sender_ptr: *mut c_void) {
    let sender =
        unsafe { Box::from_raw(sender_ptr as *mut oneshot::Sender<T>) };
    drop(sender);
}

/// A generic callback for spdk async functions expecting to be called with
/// single argument which is a sender channel to notify the other end about
/// the result.
///
/// # Arguments
///
/// * `sender_ptr`: TODO
/// * `val`: TODO
pub extern "C" fn done_cb<T>(sender_ptr: *mut c_void, val: T)
where
    T: std::fmt::Debug,
{
    let sender =
        unsafe { Box::from_raw(sender_ptr as *mut oneshot::Sender<T>) };

    // the receiver side might be gone, if this happens it either means that the
    // function has gone out of scope or that the future was cancelled. We can
    // not cancel futures as they are driven by reactor. We currently fail
    // hard if the receiver is gone but in reality the determination of it
    // being fatal depends largely on what the future was supposed to do.
    sender
        .send(val)
        .expect("done callback receiver side disappeared");
}

/// Callback for spdk async functions called with errno value.
/// Special case of the more general done_cb() above. The advantage being
/// that it converts the errno value to Result before it is sent, so the
/// receiver can use receiver.await.expect(...)? notation for processing
/// the result.
///
/// # Arguments
///
/// * `sender_ptr`: TODO
/// * `errno`: TODO
pub extern "C" fn done_errno_cb(sender_ptr: *mut c_void, errno: i32) {
    let sender = unsafe {
        Box::from_raw(sender_ptr as *mut oneshot::Sender<ErrnoResult<()>>)
    };

    sender
        .send(errno_result_from_i32((), errno))
        .expect("done callback receiver side disappeared");
}

/// Utility function for converting i32 errno value returned by SPDK to
/// a Result with Errno error holding the appropriate message for given
/// errno value. The idea is that callbacks should send this over the
/// channel and caller can then use just `.await.expect(...)?` expression
/// to process the result.
///
/// # Arguments
///
/// * `val`: TODO
/// * `errno`: TODO
pub fn errno_result_from_i32<T>(val: T, errno: i32) -> ErrnoResult<T> {
    if errno == 0 {
        Ok(val)
    } else {
        Err(Errno::from_i32(errno.abs()))
    }
}

/// TODO
///
/// # Arguments
///
/// * `errno`: TODO
pub fn errno_error<T>(errno: i32) -> ErrnoResult<T> {
    Err(Errno::from_i32(errno.abs()))
}

/// Helper routines to convert from FFI functions
#[allow(clippy::wrong_self_convention)]
pub trait FfiResult {
    /// TODO
    type Ok;

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `f`: TODO
    fn to_result<E: Error, F>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self) -> E,
        Self: Sized;
}

impl<T> FfiResult for *mut T {
    type Ok = NonNull<T>;

    #[inline]
    fn to_result<E: Error, F>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self) -> E,
    {
        NonNull::new(self).ok_or_else(|| f(self))
    }
}

impl<T> FfiResult for *const T {
    type Ok = *const T;

    #[inline]
    fn to_result<E: Error, F>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self) -> E,
    {
        if self.is_null() {
            Err(f(self))
        } else {
            Ok(self)
        }
    }
}

impl FfiResult for raw::c_int {
    type Ok = ();

    #[inline]
    fn to_result<E: Error + snafu::Error, F>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self) -> E,
    {
        if self == 0 {
            Ok(())
        } else {
            Err(f(self.abs()))
        }
    }
}

impl FfiResult for u32 {
    type Ok = ();

    #[inline]
    fn to_result<E: Error + snafu::Error, F>(self, f: F) -> Result<Self::Ok, E>
    where
        F: FnOnce(Self) -> E,
    {
        if self == 0 {
            Ok(())
        } else {
            Err(f(self))
        }
    }
}
