///! TODO
use crate::libspdk::{
    spdk_cpuset,
    spdk_cpuset_set_cpu,
    spdk_cpuset_zero,
    spdk_env_get_core_count,
    spdk_env_get_current_core,
    spdk_env_get_first_core,
    spdk_env_get_last_core,
    spdk_env_get_next_core,
};

/// TODO
#[derive(Debug)]
pub enum Core {
    /// TODO
    Count,
    /// TODO
    Current,
    /// TODO
    First,
    /// TODO
    Last,
}

/// TODO
#[derive(Debug)]
pub struct Cores(u32);

impl Cores {
    /// TODO
    pub fn count() -> Self {
        Cores(Self::get_core(Core::Count))
    }

    /// TODO
    pub fn first() -> u32 {
        Self::get_core(Core::First)
    }

    /// TODO
    pub fn last() -> Self {
        Cores(Self::get_core(Core::Last))
    }

    /// TODO
    pub fn current() -> u32 {
        unsafe { spdk_env_get_current_core() }
    }

    /// TODO
    pub fn id(&self) -> u32 {
        self.0
    }

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `c`: TODO
    fn get_core(c: Core) -> u32 {
        unsafe {
            match c {
                Core::Count => spdk_env_get_core_count(),
                Core::Current => spdk_env_get_current_core(),
                Core::First => spdk_env_get_first_core(),
                Core::Last => spdk_env_get_last_core(),
            }
        }
    }
}

impl IntoIterator for Cores {
    type Item = u32;
    type IntoIter = CoreIterator;

    fn into_iter(self) -> Self::IntoIter {
        CoreIterator {
            current: std::u32::MAX,
        }
    }
}

/// TODO
pub struct CoreIterator {
    /// TODO
    current: u32,
}

impl Iterator for CoreIterator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == std::u32::MAX {
            self.current = Cores::get_core(Core::First);
            return Some(self.current);
        }

        self.current = unsafe { spdk_env_get_next_core(self.current) };

        if self.current == std::u32::MAX {
            None
        } else {
            Some(self.current)
        }
    }
}

/// TODO
pub struct CpuMask(spdk_cpuset);

impl CpuMask {
    /// TODO
    pub fn new() -> Self {
        let mut mask = spdk_cpuset::default();
        unsafe { spdk_cpuset_zero(&mut mask) }
        Self(mask)
    }

    /// TODO
    ///
    /// # Arguments
    ///
    /// * `cpu`: TODO
    /// * `state`: TODO
    pub fn set_cpu(&mut self, cpu: u32, state: bool) {
        unsafe {
            spdk_cpuset_set_cpu(&mut self.0, cpu, state);
        }
    }

    /// TODO
    pub fn as_ptr(&self) -> *mut spdk_cpuset {
        &self.0 as *const _ as *mut spdk_cpuset
    }
}

impl Default for CpuMask {
    fn default() -> Self {
        Self::new()
    }
}
