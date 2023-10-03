use std::{
    ffi::c_void,
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{
    ffihelper::AsStr,
    libspdk::{
        spdk_nvmf_ctrlr,
        spdk_nvmf_subsystem_event,
        SPDK_NVMF_SUBSYSTEM_EVENT_HOST_CONNECT,
        SPDK_NVMF_SUBSYSTEM_EVENT_HOST_DISCONNECT,
        SPDK_NVMF_SUBSYSTEM_EVENT_HOST_KEEP_ALIVE_TIMEOUT,
    },
};

/// TODO
#[derive(Copy, Clone)]
#[repr(C)]
pub struct NvmfController(pub NonNull<spdk_nvmf_ctrlr>);

impl NvmfController {
    /// Get the hostnqn from the controller
    pub fn hostnqn(&self) -> String {
        unsafe { self.0.as_ref().hostnqn.as_str().to_string() }
    }
}
impl From<*mut spdk_nvmf_ctrlr> for NvmfController {
    fn from(s: *mut spdk_nvmf_ctrlr) -> Self {
        NvmfController(NonNull::new(s).unwrap())
    }
}

impl Debug for NvmfController {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("NvmfController")
            .field("hostnqn", &self.hostnqn())
            .finish()
    }
}

impl Deref for NvmfController {
    type Target = spdk_nvmf_ctrlr;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl DerefMut for NvmfController {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

/// TODO
#[derive(Copy, Clone, Debug)]
pub enum NvmfSubsystemEvent {
    HostConnect(NvmfController),
    HostDisconnect(NvmfController),
    HostKeepAliveTimeout(NvmfController),
    Unknown,
}

impl NvmfSubsystemEvent {
    pub fn from_cb_args(
        event: spdk_nvmf_subsystem_event,
        ctx: *mut c_void,
    ) -> Self {
        match event {
            SPDK_NVMF_SUBSYSTEM_EVENT_HOST_CONNECT => Self::HostConnect(
                NvmfController::from(ctx as *mut spdk_nvmf_ctrlr),
            ),
            SPDK_NVMF_SUBSYSTEM_EVENT_HOST_DISCONNECT => Self::HostDisconnect(
                NvmfController::from(ctx as *mut spdk_nvmf_ctrlr),
            ),
            SPDK_NVMF_SUBSYSTEM_EVENT_HOST_KEEP_ALIVE_TIMEOUT => {
                Self::HostKeepAliveTimeout(NvmfController::from(
                    ctx as *mut spdk_nvmf_ctrlr,
                ))
            }
            _ => {
                warn!("Unknown NVMF subsystem event: {event:?}");
                Self::Unknown
            }
        }
    }
}
