///! TODO
use crate::libspdk::{
    spdk_bdev_io,
    spdk_bdev_io_get_nvme_status,
    spdk_nvme_cpl,
};

/// Corresponds to `spdk_nvme_status`, `spdk_nvme_status_code_type`.
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
pub enum NvmeStatus {
    Generic(GenericStatusCode),
    CommandSpecific(CommandSpecificStatusCode),
    MediaError(MediaErrorStatusCode),
    Path(PathStatusCode),
    VendorSpecific(i32),
    Reserved(i32),
    Unknown(i32),
}

impl From<(i32, i32)> for NvmeStatus {
    fn from(s: (i32, i32)) -> Self {
        let sct = s.0;
        let sc = s.1;

        match sct {
            0x00 => Self::Generic(GenericStatusCode::from(sc)),
            0x01 => Self::CommandSpecific(CommandSpecificStatusCode::from(sc)),
            0x02 => Self::MediaError(MediaErrorStatusCode::from(sc)),
            0x03 => Self::Path(PathStatusCode::from(sc)),
            0x04 | 0x05 | 0x06 => Self::Reserved(sc),
            0x07 => Self::VendorSpecific(sc),
            _ => Self::Unknown(sc),
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
        let (r) = unsafe {
            let cplr = &*cpl;

            (
                cplr.__bindgen_anon_1.status.sct().into(),
                cplr.__bindgen_anon_1.status.sc().into(),
            )
        };

        Self::from(r)
    }
}

/// Generic command status codes.
/// Corresponds to `spdk_nvme_generic_command_status_code`.
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum GenericStatusCode {
    Success,
    InvalidOpcode,
    InvalidFieldInCommand,
    CommandIDConflict,
    DataTransferError,
    CommandsAbortedDueToPowerLoss,
    InternalDeviceError,
    AbortedRequested,
    AbortedSubmissionQueueDeleted,
    AbortedSubmissionFailedFusedCommand,
    AbortedSubmissionMissingFusedCommand,
    InvalidNameSpaceOrFormat,
    CommandSequenceError,
    InvalidSGLDescriptor,
    InvalidNumberOfSGLDescriptors,
    DataSGLLengthInvalid,
    MetaDataSGLLengthInvalid,
    SGLTypeDescriptorInvalid,
    InvalidUseOfControlMemoryBuffer,
    PRPOffsetInvalid,
    AtomicWriteUnitExceeded,
    OperationDenied,
    SGLOffsetInvalid,
    HostIdentifierInvalidFormat,
    KATOExpired,
    KATOInvalid,
    CommandAbortPreemt,
    SanitizeFailed,
    SanitizeInProgress,
    SGLDataBlockGranularityInvalid,
    CommandInvalidInCMB,
    LBAOutOfRange,
    CapacityExceeded,
    NamespaceNotReady,
    ReservationConflict,
    FormatInProgress,
    Reserved,
}

impl From<i32> for GenericStatusCode {
    fn from(i: i32) -> Self {
        match i {
            0x00 => Self::Success,
            0x01 => Self::InvalidOpcode,
            0x02 => Self::InvalidFieldInCommand,
            0x03 => Self::CommandIDConflict,
            0x04 => Self::DataTransferError,
            0x05 => Self::CommandsAbortedDueToPowerLoss,
            0x06 => Self::InternalDeviceError,
            0x07 => Self::AbortedRequested,
            0x08 => Self::AbortedSubmissionQueueDeleted,
            0x09 => Self::AbortedSubmissionFailedFusedCommand,
            0x0A => Self::AbortedSubmissionMissingFusedCommand,
            0x0B => Self::InvalidNameSpaceOrFormat,
            0x0C => Self::CommandSequenceError,
            0x0D => Self::InvalidSGLDescriptor,
            0x0E => Self::InvalidSGLDescriptor,
            0x0F => Self::DataSGLLengthInvalid,
            0x10 => Self::MetaDataSGLLengthInvalid,
            0x11 => Self::SGLTypeDescriptorInvalid,
            0x12 => Self::InvalidUseOfControlMemoryBuffer,
            0x13 => Self::PRPOffsetInvalid,
            0x14 => Self::AtomicWriteUnitExceeded,
            0x15 => Self::OperationDenied,
            0x16 => Self::SGLOffsetInvalid,
            0x17 => Self::Reserved,
            0x18 => Self::HostIdentifierInvalidFormat,
            0x19 => Self::KATOExpired,
            0x1A => Self::KATOInvalid,
            0x1B => Self::CommandAbortPreemt,
            0x1C => Self::SanitizeFailed,
            0x1D => Self::SanitizeInProgress,
            0x1E => Self::SGLDataBlockGranularityInvalid,
            0x1F => Self::CommandInvalidInCMB,
            0x80 => Self::LBAOutOfRange,
            0x81 => Self::CapacityExceeded,
            0x82 => Self::NamespaceNotReady,
            0x83 => Self::ReservationConflict,
            0x84 => Self::FormatInProgress,
            _ => {
                error!("unknown code {:x}", i);
                Self::Reserved
            }
        }
    }
}

/// Command specific status codes.
/// Corresponds to `spdk_nvme_command_specific_status_code`.
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
pub enum CommandSpecificStatusCode {
    CompletionQueueInvalid,
    InvalidQueueIdentifier,
    InvalidQueueSize,
    AbortCommandLimitExceeded,
    AsyncEventRequestLimitExceeded,
    InvalidFirmwareSlot,
    InvalidFirmwareImage,
    InvalidInterruptVector,
    InvalidLogPage,
    InvalidFormat,
    FirmwareReqConventionalReset,
    InvalidQueueDeletion,
    FeatureIdNotSaveable,
    FeatureNotChangeable,
    FeatureNotNamespaceSpecific,
    FirmwareReqNvmReset,
    FirmwareReqReset,
    FirmwareReqMaxTimeViolation,
    FirmwareActivationProhibited,
    OverlappingRange,
    NamespaceInsufficientCapacity,
    NamespaceIdUnavailable,
    NamespaceAlreadyAttached,
    NamespaceIsPrivate,
    NamespaceNotAttached,
    ThinprovisioningNotSupported,
    ControllerListInvalid,
    DeviceSelfTestInProgress,
    BootPartitionWriteProhibited,
    InvalidCtrlrId,
    InvalidSecondaryCtrlrState,
    InvalidNumCtrlrResources,
    InvalidResourceId,
    IocsNotSupported,
    IocsNotEnabled,
    IocsCombinationRejected,
    InvalidIocs,
    StreamResourceAllocationFailed,
    ConflictingAttributes,
    InvalidProtectionInfo,
    AttemptedWriteToRoRange,
    CmdSizeLimitSizeExceeded,
    Unknown,
}

impl From<i32> for CommandSpecificStatusCode {
    fn from(i: i32) -> Self {
        match i {
            0x00 => Self::CompletionQueueInvalid,
            0x01 => Self::InvalidQueueIdentifier,
            0x02 => Self::InvalidQueueSize,
            0x03 => Self::AbortCommandLimitExceeded,
            0x05 => Self::AsyncEventRequestLimitExceeded,
            0x06 => Self::InvalidFirmwareSlot,
            0x07 => Self::InvalidFirmwareImage,
            0x08 => Self::InvalidInterruptVector,
            0x09 => Self::InvalidLogPage,
            0x0a => Self::InvalidFormat,
            0x0b => Self::FirmwareReqConventionalReset,
            0x0c => Self::InvalidQueueDeletion,
            0x0d => Self::FeatureIdNotSaveable,
            0x0e => Self::FeatureNotChangeable,
            0x0f => Self::FeatureNotNamespaceSpecific,
            0x10 => Self::FirmwareReqNvmReset,
            0x11 => Self::FirmwareReqReset,
            0x12 => Self::FirmwareReqMaxTimeViolation,
            0x13 => Self::FirmwareActivationProhibited,
            0x14 => Self::OverlappingRange,
            0x15 => Self::NamespaceInsufficientCapacity,
            0x16 => Self::NamespaceIdUnavailable,
            0x18 => Self::NamespaceAlreadyAttached,
            0x19 => Self::NamespaceIsPrivate,
            0x1a => Self::NamespaceNotAttached,
            0x1b => Self::ThinprovisioningNotSupported,
            0x1c => Self::ControllerListInvalid,
            0x1d => Self::DeviceSelfTestInProgress,
            0x1e => Self::BootPartitionWriteProhibited,
            0x1f => Self::InvalidCtrlrId,
            0x20 => Self::InvalidSecondaryCtrlrState,
            0x21 => Self::InvalidNumCtrlrResources,
            0x22 => Self::InvalidResourceId,
            0x29 => Self::IocsNotSupported,
            0x2a => Self::IocsNotEnabled,
            0x2b => Self::IocsCombinationRejected,
            0x2c => Self::InvalidIocs,
            0x7f => Self::StreamResourceAllocationFailed,
            0x80 => Self::ConflictingAttributes,
            0x81 => Self::InvalidProtectionInfo,
            0x82 => Self::AttemptedWriteToRoRange,
            0x83 => Self::CmdSizeLimitSizeExceeded,
            _ => Self::Unknown,
        }
    }
}

/// Media error status codes
/// Corresponds to `spdk_nvme_media_error_status_code`.
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
pub enum MediaErrorStatusCode {
    WriteFaults,
    UnrecoveredReadError,
    GuardCheckError,
    ApplicationTagCheckError,
    ReferenceTagCheckError,
    CompareFailure,
    AccessDenied,
    DeallocatedOrUnwrittenBlock,
    Unknown,
}

impl From<i32> for MediaErrorStatusCode {
    fn from(i: i32) -> Self {
        match i {
            0x80 => Self::WriteFaults,
            0x81 => Self::UnrecoveredReadError,
            0x82 => Self::GuardCheckError,
            0x83 => Self::ApplicationTagCheckError,
            0x84 => Self::ReferenceTagCheckError,
            0x85 => Self::CompareFailure,
            0x86 => Self::AccessDenied,
            0x87 => Self::DeallocatedOrUnwrittenBlock,
            _ => Self::Unknown,
        }
    }
}

/// Path related status codes.
/// Corresponds to `spdk_nvme_path_status_code`.
#[derive(Debug, Copy, Clone, Eq, PartialOrd, PartialEq)]
pub enum PathStatusCode {
    InternalPathError,
    AsymmetricAccessPersistentLoss,
    AsymmetricAccessInaccessible,
    AsymmetricAccessTransition,
    ControllerPathError,
    HostPathError,
    AbortedByHost,
    Unknown,
}

impl From<i32> for PathStatusCode {
    fn from(i: i32) -> Self {
        match i {
            0x00 => Self::InternalPathError,
            0x01 => Self::AsymmetricAccessPersistentLoss,
            0x02 => Self::AsymmetricAccessInaccessible,
            0x03 => Self::AsymmetricAccessTransition,
            0x60 => Self::ControllerPathError,
            0x70 => Self::HostPathError,
            0x71 => Self::AbortedByHost,
            _ => Self::Unknown,
        }
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
    pub const CREATE_SNAPSHOT: u8 = 0xc0;
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
    // pub const RESERVATION_RELEASE: u8 = 0x15;
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
    pub const PERSIST_POWER_LOSS: u8 = 0x2;
}

/// TODO
pub mod nvme_reservation_acquire_action {
    pub const ACQUIRE: u8 = 0x0;
    pub const PREEMPT: u8 = 0x1;
    pub const PREEMPT_ABORT: u8 = 0x2;
}
