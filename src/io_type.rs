///! TODO
use super::libspdk;

/// TODO
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
#[repr(u32)]
pub enum IoType {
    Invalid = libspdk::SPDK_BDEV_IO_TYPE_INVALID,
    Read = libspdk::SPDK_BDEV_IO_TYPE_READ,
    Write = libspdk::SPDK_BDEV_IO_TYPE_WRITE,
    Unmap = libspdk::SPDK_BDEV_IO_TYPE_UNMAP,
    Flush = libspdk::SPDK_BDEV_IO_TYPE_FLUSH,
    Reset = libspdk::SPDK_BDEV_IO_TYPE_RESET,
    NvmeAdmin = libspdk::SPDK_BDEV_IO_TYPE_NVME_ADMIN,
    NvmeIo = libspdk::SPDK_BDEV_IO_TYPE_NVME_IO,
    NvmeIoMd = libspdk::SPDK_BDEV_IO_TYPE_NVME_IO_MD,
    WriteZeros = libspdk::SPDK_BDEV_IO_TYPE_WRITE_ZEROES,
    ZeroCopy = libspdk::SPDK_BDEV_IO_TYPE_ZCOPY,
    ZoneInfo = libspdk::SPDK_BDEV_IO_TYPE_GET_ZONE_INFO,
    ZoneManagement = libspdk::SPDK_BDEV_IO_TYPE_ZONE_MANAGEMENT,
    ZoneAppend = libspdk::SPDK_BDEV_IO_TYPE_ZONE_APPEND,
    Compare = libspdk::SPDK_BDEV_IO_TYPE_COMPARE,
    CompareAndWrite = libspdk::SPDK_BDEV_IO_TYPE_COMPARE_AND_WRITE,
    Abort = libspdk::SPDK_BDEV_IO_TYPE_ABORT,
    SeekHole = libspdk::SPDK_BDEV_IO_TYPE_SEEK_HOLE,
    SeekData = libspdk::SPDK_BDEV_IO_TYPE_SEEK_DATA,
    Copy = libspdk::SPDK_BDEV_IO_TYPE_COPY,
    IoNumTypes = libspdk::SPDK_BDEV_NUM_IO_TYPES,
}

/// TODO
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Eq)]
#[non_exhaustive]
#[repr(i32)]
pub enum IoStatus {
    AioError = libspdk::SPDK_BDEV_IO_STATUS_AIO_ERROR,
    Aborted = libspdk::SPDK_BDEV_IO_STATUS_ABORTED,
    FirstFusedFailed = libspdk::SPDK_BDEV_IO_STATUS_FIRST_FUSED_FAILED,
    MisCompared = libspdk::SPDK_BDEV_IO_STATUS_MISCOMPARE,
    NoMemory = libspdk::SPDK_BDEV_IO_STATUS_NOMEM,
    ScsiError = libspdk::SPDK_BDEV_IO_STATUS_SCSI_ERROR,
    NvmeError = libspdk::SPDK_BDEV_IO_STATUS_NVME_ERROR,
    Failed = libspdk::SPDK_BDEV_IO_STATUS_FAILED,
    Pending = libspdk::SPDK_BDEV_IO_STATUS_PENDING,
    Success = libspdk::SPDK_BDEV_IO_STATUS_SUCCESS,
}

impl From<IoType> for u32 {
    fn from(t: IoType) -> Self {
        t as u32
    }
}

impl From<u32> for IoType {
    fn from(u: u32) -> Self {
        assert!(
            u <= libspdk::SPDK_BDEV_NUM_IO_TYPES,
            "Invalid or unknown I/O type"
        );
        unsafe { *std::mem::transmute::<*const u32, *const IoType>(&u) }
    }
}

impl From<i32> for IoStatus {
    fn from(s: i32) -> Self {
        assert!(
            s >= libspdk::SPDK_MIN_BDEV_IO_STATUS
                && s <= libspdk::SPDK_BDEV_IO_STATUS_SUCCESS,
            "Invalid or unknown status code"
        );
        unsafe { *std::mem::transmute::<*const i32, *const IoStatus>(&s) }
    }
}

impl From<IoStatus> for i32 {
    fn from(i: IoStatus) -> Self {
        i as i32
    }
}

impl From<i8> for IoStatus {
    fn from(status: i8) -> Self {
        (status as i32).into()
    }
}
