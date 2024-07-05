use futures::channel::oneshot::{self, Canceled};
use nix::errno::Errno;
use std::mem::zeroed;

use crate::{
    ffihelper::{
        cb_arg,
        copy_str_with_null,
        done_errno_cb,
        drop_cb_arg,
        ErrnoResult,
        IntoCString,
    },
    libspdk::{bdev_nvme_delete, nvme_path_id, SPDK_NVME_TRANSPORT_PCIE},
};

/// Async wrapper for `bdev_nvme_delete`.
/// `bdev_nvme_delete` differs from other bdev_*_delete function family,
/// as it may return errno instead calling the callback (which is optional).
///
/// # Arguments
///
/// * `name`: Controller name.
///
/// * `path_id`: Controller path ID. If not given, `name` is used for traddr,
///   and SPDK_NVME_TRANSPORT_PCIE for trtype.
pub async fn bdev_nvme_delete_async(
    name: &str,
    path_id: Option<nvme_path_id>,
) -> Result<ErrnoResult<()>, Canceled> {
    let path_id = path_id.unwrap_or_else(|| {
        let mut path_id = unsafe {
            nvme_path_id {
                trid: zeroed(),
                hostid: zeroed(),
                link: zeroed(),
                last_failed_tsc: 0,
            }
        };
        copy_str_with_null(name, &mut path_id.trid.traddr);
        path_id.trid.trtype = SPDK_NVME_TRANSPORT_PCIE;
        path_id
    });

    let (s, r) = oneshot::channel::<ErrnoResult<()>>();
    let arg = cb_arg(s);

    let errno = unsafe {
        bdev_nvme_delete(
            name.to_string().into_cstring().as_ptr(),
            &path_id,
            Some(done_errno_cb),
            arg,
        )
    };

    // `bdev_nvme_delete` failed to run: callback won't be called.
    if errno < 0 {
        drop_cb_arg::<()>(arg);
        return Ok(Err(Errno::from_i32(-errno)));
    }

    r.await
}
