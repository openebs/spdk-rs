use nix::errno::Errno;
use std::{
    fmt::{Debug, Formatter},
    mem::transmute,
};

use crate::libspdk::{
    spdk_bdev_io,
    spdk_bdev_io_get_nvme_status,
    spdk_nvme_command_specific_status_code,
    spdk_nvme_cpl,
    spdk_nvme_generic_command_status_code,
    spdk_nvme_media_error_status_code,
    spdk_nvme_path_status_code,
    spdk_nvme_status,
    spdk_nvme_status_code_type,
    spdk_nvmf_request,
    SPDK_NVME_SCT_COMMAND_SPECIFIC,
    SPDK_NVME_SCT_GENERIC,
    SPDK_NVME_SCT_MEDIA_ERROR,
    SPDK_NVME_SCT_PATH,
    SPDK_NVME_SCT_VENDOR_SPECIFIC,
    SPDK_NVME_SC_DATA_TRANSFER_ERROR,
    SPDK_NVME_SC_DEALLOCATED_OR_UNWRITTEN_BLOCK,
    SPDK_NVME_SC_SUCCESS,
};

/// Accessors for `spdk_nvme_cpl` (completion queue entry) struct.
impl spdk_nvme_cpl {
    /// Returns NVME status word.
    pub fn status(&self) -> spdk_nvme_status {
        unsafe { self.__bindgen_anon_1.status }
    }

    /// Sets NVME status word.
    pub fn set_status(&mut self, status: spdk_nvme_status) {
        unsafe {
            self.__bindgen_anon_1.status = status;
        }
    }
}

/// Accessors for `spdk_nvmf_request` struct.
impl spdk_nvmf_request {
    /// Returns a reference to the request's completion queue entry.
    pub fn nvme_cpl(&self) -> &spdk_nvme_cpl {
        unsafe { &((*self.rsp).nvme_cpl) }
    }

    /// Returns a mutable reference to the request's completion queue entry.
    pub fn nvme_cpl_mut(&mut self) -> &mut spdk_nvme_cpl {
        // spdk_nvme_power_state
        unsafe { &mut ((*self.rsp).nvme_cpl) }
    }
}

/// Accessors for `spdk_nvme_status` struct.
impl spdk_nvme_status {
    /// Converts self to `NvmeStatus`.
    pub fn status(&self) -> NvmeStatus {
        NvmeStatus::from(*self)
    }
}

/// Status code types.
#[derive(Copy, Clone, Eq, PartialOrd, PartialEq)]
pub enum NvmeStatus {
    /// Generic command status codes.
    /// Corresponds to `spdk_nvme_generic_command_status_code` grouping.
    Generic(spdk_nvme_generic_command_status_code),

    /// Command specific status codes.
    /// Corresponds to `spdk_nvme_command_specific_status_code` grouping.
    CmdSpecific(spdk_nvme_command_specific_status_code),

    /// Media error status codes.
    /// Corresponds to `spdk_nvme_media_error_status_code` grouping.
    Media(spdk_nvme_media_error_status_code),

    /// Path related status codes.
    /// Corresponds to `spdk_nvme_path_status_code` grouping.
    Path(spdk_nvme_path_status_code),

    /// Vendor-specific codes.
    VendorSpecific(i32),

    /// Unknown code.
    Unknown(i32, i32),
}

impl NvmeStatus {
    /// Shorthand for a success code.
    pub const SUCCESS: Self = NvmeStatus::Generic(SPDK_NVME_SC_SUCCESS);

    /// Shorthand for SPDK_NVME_SC_DEALLOCATED_OR_UNWRITTEN_BLOCK.
    pub const UNWRITTEN_BLOCK: Self =
        Self::Media(SPDK_NVME_SC_DEALLOCATED_OR_UNWRITTEN_BLOCK);

    /// Shorthand for a vendor-specific ENOSPC error.
    pub const NO_SPACE: Self = Self::VendorSpecific(Errno::ENOSPC as i32);

    /// A shorthand for a generic data transfer error.
    pub const DATA_TRANSFER_ERROR: Self =
        NvmeStatus::Generic(SPDK_NVME_SC_DATA_TRANSFER_ERROR);

    /// TODO
    pub fn as_sct_sc_codes(&self) -> (i32, i32) {
        unsafe {
            match *self {
                Self::Generic(c) => {
                    (transmute(SPDK_NVME_SCT_GENERIC), transmute(c))
                }
                Self::CmdSpecific(c) => {
                    (transmute(SPDK_NVME_SCT_COMMAND_SPECIFIC), transmute(c))
                }
                Self::Media(c) => {
                    (transmute(SPDK_NVME_SCT_MEDIA_ERROR), transmute(c))
                }
                Self::Path(c) => (transmute(SPDK_NVME_SCT_PATH), transmute(c)),
                Self::VendorSpecific(c) => {
                    (transmute(SPDK_NVME_SCT_VENDOR_SPECIFIC), transmute(c))
                }
                Self::Unknown(sct, sc) => (transmute(sct), transmute(sc)),
            }
        }
    }

    /// Determines if this status is a success code.
    #[inline(always)]
    pub fn is_success(&self) -> bool {
        *self == Self::SUCCESS
    }

    /// Determines if this status is a vendor-specific ENOSPC error.
    #[inline(always)]
    pub fn is_no_space(&self) -> bool {
        *self == Self::NO_SPACE
    }
}

impl Debug for NvmeStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NvmeStatus::Generic(s) => write!(f, "{s:?}"),
            NvmeStatus::CmdSpecific(s) => write!(f, "{s:?}"),
            NvmeStatus::Media(s) => write!(f, "{s:?}"),
            NvmeStatus::Path(s) => write!(f, "{s:?}"),
            NvmeStatus::VendorSpecific(s) => {
                let s = *s;
                let en = if s > 0 {
                    Errno::from_i32(s)
                } else {
                    Errno::UnknownErrno
                };

                if en == Errno::UnknownErrno {
                    write!(f, "SPDK_NVME_SCT_VENDOR_SPECIFIC ({s})")
                } else {
                    write!(f, "SPDK_NVME_SCT_VENDOR_SPECIFIC ({en}: {s})")
                }
            }
            NvmeStatus::Unknown(sct, sc) => write!(f, "UNKNOWN ({sct},{sc})"),
        }
    }
}

/// Converts `spdk_nvme_status` into `NvmeStatus`.
impl From<spdk_nvme_status> for NvmeStatus {
    fn from(s: spdk_nvme_status) -> Self {
        Self::from((s.sct().into(), s.sc().into()))
    }
}

/// Converts a (sct, sc) pair into `NvmeStatus`.
impl From<(i32, i32)> for NvmeStatus {
    fn from(s: (i32, i32)) -> Self {
        unsafe {
            let (sct, sc) = s;

            match transmute(sct) {
                SPDK_NVME_SCT_GENERIC => Self::Generic(transmute(sc)),
                SPDK_NVME_SCT_COMMAND_SPECIFIC => {
                    Self::CmdSpecific(transmute(sc))
                }
                SPDK_NVME_SCT_MEDIA_ERROR => Self::Media(transmute(sc)),
                SPDK_NVME_SCT_PATH => Self::Path(transmute(sc)),
                SPDK_NVME_SCT_VENDOR_SPECIFIC => Self::VendorSpecific(sc),
                _ => Self::Unknown(sct, sc),
            }
        }
    }
}

impl From<*mut spdk_bdev_io> for NvmeStatus {
    fn from(b: *mut spdk_bdev_io) -> Self {
        let mut cdw0: u32 = 0;
        let mut sct: i32 = 0;
        let mut sc: i32 = 0;

        unsafe { spdk_bdev_io_get_nvme_status(b, &mut cdw0, &mut sct, &mut sc) }

        Self::from((sct, sc))
    }
}

impl From<*const spdk_nvme_cpl> for NvmeStatus {
    fn from(cpl: *const spdk_nvme_cpl) -> Self {
        unsafe { Self::from((*cpl).status()) }
    }
}

/// NVMe Admin opcode, from nvme_spec.h
pub mod nvme_admin_opc {
    // pub const GET_LOG_PAGE: u8 = 0x02;
    pub const IDENTIFY: u8 = 0x06;
    // pub const ABORT: u8 = 0x08;
    // pub const SET_FEATURES: u8 = 0x09;
    // pub const GET_FEATURES: u8 = 0x0a;
    // Vendor-specific
    pub const CREATE_SNAPSHOT: u8 = 0xc1;
}

/// NVM command set opcodes, from nvme_spec.h
pub mod nvme_nvm_opcode {
    // pub const FLUSH: u8 = 0x00;
    // pub const WRITE: u8 = 0x01;
    // pub const READ: u8 = 0x02;
    // pub const WRITE_UNCORRECTABLE: u8 = 0x04;
    // pub const COMPARE: u8 = 0x05;
    // pub const WRITE_ZEROES: u8 = 0x08;
    // pub const DATASET_MANAGEMENT: u8 = 0x09;
    pub const RESERVATION_REGISTER: u8 = 0x0d;
    pub const RESERVATION_REPORT: u8 = 0x0e;
    pub const RESERVATION_ACQUIRE: u8 = 0x11;
    pub const RESERVATION_RELEASE: u8 = 0x15;
}

/// TODO
pub mod nvme_reservation_type {
    pub const WRITE_EXCLUSIVE: u8 = 0x1;
    pub const EXCLUSIVE_ACCESS: u8 = 0x2;
    pub const WRITE_EXCLUSIVE_REG_ONLY: u8 = 0x3;
    pub const EXCLUSIVE_ACCESS_REG_ONLY: u8 = 0x4;
    pub const WRITE_EXCLUSIVE_ALL_REGS: u8 = 0x5;
    pub const EXCLUSIVE_ACCESS_ALL_REGS: u8 = 0x6;
}

/// TODO
pub mod nvme_reservation_register_action {
    pub const REGISTER_KEY: u8 = 0x0;
    pub const UNREGISTER_KEY: u8 = 0x1;
    pub const REPLACE_KEY: u8 = 0x2;
}

/// TODO
pub mod nvme_reservation_register_cptpl {
    pub const NO_CHANGES: u8 = 0x0;
    pub const CLEAR_POWER_ON: u8 = 0x2;
    pub const PERSIST_POWER_LOSS: u8 = 0x3;
}

/// TODO
pub mod nvme_reservation_acquire_action {
    pub const ACQUIRE: u8 = 0x0;
    pub const PREEMPT: u8 = 0x1;
    pub const PREEMPT_ABORT: u8 = 0x2;
}
