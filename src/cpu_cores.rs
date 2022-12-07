use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
};

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

    /// Returns list of CPU cores.
    pub fn list_cores() -> Vec<u32> {
        Self::count().into_iter().collect()
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

/// Base CPU core selector.
pub struct CoreSelectorBase {
    cores: Vec<u32>,
    next: usize,
}

impl Debug for CoreSelectorBase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.cores)
    }
}

impl CoreSelectorBase {
    /// Creates new core selector base.
    pub fn new() -> Self {
        let cores = Cores::list_cores();
        assert!(cores.len() > 0, "No CPU cores found");
        Self {
            cores,
            next: 0,
        }
    }
}

/// Round-robin core select.
pub struct RoundRobinCoreSelector(CoreSelectorBase);

impl Deref for RoundRobinCoreSelector {
    type Target = CoreSelectorBase;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RoundRobinCoreSelector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl RoundRobinCoreSelector {
    /// Creates new round-robin core selector.
    pub fn new() -> Self {
        Self(CoreSelectorBase::new())
    }

    /// Selects the next core, filtering out the unsuitable ones.
    pub fn filter_next(&mut self, mut f: impl FnMut(u32) -> bool) -> u32 {
        let mut n = self.next;
        let start = n;

        loop {
            self.next += 1;
            if self.next == self.cores.len() {
                self.next = 0;
            }

            if f(self.cores[n]) {
                break;
            }

            n = self.next;
            if start == n {
                break;
            }
        }

        self.cores[n]
    }
}
