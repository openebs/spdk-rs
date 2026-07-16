use futures::channel::oneshot::{channel, Receiver, Sender};
use nix::{errno::Errno, libc};
use std::{
    ffi::{c_void, CStr, CString},
    fmt::{Debug, Formatter},
    future::Future,
    ptr::NonNull,
};

use crate::{
    cpu_cores::{Cores, CpuMask},
    libspdk::{
        spdk_event_handler_opts, spdk_fd_group, spdk_fd_group_add, spdk_fd_group_add_ext,
        spdk_fd_group_create, spdk_fd_group_destroy, spdk_fd_group_get_default_event_handler_opts,
        spdk_fd_group_nest, spdk_fd_group_unnest, spdk_fd_group_wait, spdk_get_thread,
        spdk_interrupt_mode_enable, spdk_interrupt_mode_is_enabled, spdk_set_thread, spdk_thread,
        spdk_thread_create, spdk_thread_destroy, spdk_thread_exit, spdk_thread_get_by_id,
        spdk_thread_get_id, spdk_thread_get_interrupt_fd, spdk_thread_get_interrupt_fd_group,
        spdk_thread_get_name, spdk_thread_is_exited, spdk_thread_poll, spdk_thread_send_msg,
        spdk_thread_set_interrupt_mode,
    },
};

/// The process's baseline CPU affinity: under a cpuset cgroup this is the
/// container's `cpuset.cpus` (the CPUs assigned to this workload). It is the
/// correct seed for placing off-reactor worker threads: unlike a thread's
/// *current* affinity, it does not collapse to a single reactor core when
/// `unaffinitize` is invoked from within a reactor context.
///
/// Seeded once at startup via [`Thread::capture_base_cpuset`], and refreshed
/// via [`Thread::refresh_base_cpuset`] whenever kubelet's CPUManager grows or
/// shrinks the container cpuset -- so restored worker affinity always tracks
/// the *current* cpuset, and workers reclaim CPUs that are returned to the
/// workload rather than staying pinned to the smaller startup set.
static BASE_CPUSET: std::sync::Mutex<Option<libc::cpu_set_t>> = std::sync::Mutex::new(None);

/// Wrapper for `spdk_thread`.
#[derive(PartialEq, Clone, Copy)]
pub struct Thread {
    inner: NonNull<spdk_thread>,
}

unsafe impl Send for Thread {}

impl Debug for Thread {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            write!(
                f,
                "{core}::{name} ({addr:p})",
                core = Cores::current(),
                name = self.name(),
                addr = self.as_ptr(),
            )
        } else {
            write!(
                f,
                "{core}::{name}",
                core = Cores::current(),
                name = self.name(),
            )
        }
    }
}

impl Thread {
    /// With the given thread as context, execute the closure on that thread.
    ///
    /// Any function can be executed here however, this should typically be used
    /// to execute functions that reference any FFI to SPDK.
    pub fn new(name: String, core: u32) -> Option<Self> {
        let name = CString::new(name).unwrap();

        NonNull::new(unsafe {
            let mut mask = CpuMask::new();
            mask.set_cpu(core, true);
            spdk_thread_create(name.as_ptr(), mask.as_ptr())
        })
        .map(|inner| Self { inner })
    }

    /// Find thread by its thread id.
    pub fn by_id(id: u64) -> Option<Self> {
        NonNull::new(unsafe { spdk_thread_get_by_id(id) }).map(|inner| Self { inner })
    }

    /// Marks thread as exiting.
    pub fn exit(&self) {
        trace!("Exiting SPDK thread: {:?}", self);

        let _g = CurrentThreadGuard::new();
        self.set_current();
        unsafe {
            spdk_thread_exit(self.as_ptr());
        }
    }

    /// Marks a thread as exiting, and waits until it exits by polling it.
    pub fn wait_exit(&self) {
        trace!("Waiting SPDK thread to exit: {:?}", self);

        let _g = CurrentThreadGuard::new();

        self.set_current();

        unsafe {
            spdk_thread_exit(self.as_ptr());

            // now wait until the thread is actually exited the internal
            // state is updated by spdk_thread_poll()
            while !spdk_thread_is_exited(self.as_ptr()) {
                spdk_thread_poll(self.as_ptr(), 0, 0);
            }
        }
    }

    /// Destroys a thread, freeing all of its resources.
    /// Only an exited thread can be safely destroyed, so client code
    /// must ensure the thread has exited before destroying it.
    pub fn destroy(self) {
        trace!("Destroying SPDK thread: {:?}", self);

        assert!(self.is_exited());

        let _g = CurrentThreadGuard::new();

        unsafe {
            spdk_thread_destroy(self.as_ptr());
        }
    }

    /// Gets a handle to the current thread.
    /// Returns an SPDK thread wrapper instance if this is an SPDK thread,
    /// or `None` otherwise.
    pub fn current() -> Option<Self> {
        let thread = unsafe { spdk_get_thread() };
        if thread.is_null() {
            None
        } else {
            Some(Self::from_ptr(thread))
        }
    }

    /// Returns the primary ("init") SPDK thread.
    pub fn primary() -> Self {
        Self {
            inner: NonNull::new(unsafe { spdk_thread_get_by_id(1) })
                .expect("No init thread allocated"),
        }
    }

    /// Returns the primary ("init") SPDK thread or None.
    /// Useful when shutting down before init thread is allocated.
    pub fn primary_safe() -> Option<Self> {
        NonNull::new(unsafe { spdk_thread_get_by_id(1) }).map(|inner| Self { inner })
    }

    /// Returns thread identifier.
    pub fn id(&self) -> u64 {
        unsafe { spdk_thread_get_id(self.as_ptr()) }
    }

    /// Returns thread name.
    pub fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr(spdk_thread_get_name(self.as_ptr()))
                .to_str()
                .unwrap()
        }
    }

    /// TODO
    #[inline]
    pub fn poll(&self) {
        let _ = unsafe { spdk_thread_poll(self.as_ptr(), 0, 0) };
    }

    /// Switch the current SPDK thread between poll mode and interrupt mode.
    ///
    /// In interrupt mode, the thread's pollers are driven by epoll events
    /// instead of busy-polling, dramatically reducing CPU usage when idle.
    ///
    /// # Safety
    /// Must be called from within the context of this SPDK thread
    /// (i.e., this thread must be set as the current thread).
    /// `interrupt_mode_enable()` must have been called during init.
    pub fn set_interrupt_mode(enable: bool) {
        unsafe { spdk_thread_set_interrupt_mode(enable) }
    }

    /// Get the interrupt fd (epoll fd) for this thread.
    ///
    /// Returns the file descriptor that becomes ready when any of the
    /// thread's interrupt file descriptors have events. Only meaningful
    /// when the thread is in interrupt mode.
    pub fn get_interrupt_fd(&self) -> i32 {
        unsafe { spdk_thread_get_interrupt_fd(self.as_ptr()) }
    }

    /// Enable SPDK interrupt mode globally.
    ///
    /// Must be called once during initialization before any thread
    /// can use `set_interrupt_mode()`.
    ///
    /// Returns `Ok(())` on success. Must be called before
    /// `spdk_thread_lib_init_ext()`.
    pub fn interrupt_mode_enable() -> Result<(), Errno> {
        let rc = unsafe { spdk_interrupt_mode_enable() };
        if rc != 0 {
            Err(Errno::from_raw(rc.abs()))
        } else {
            Ok(())
        }
    }

    /// Check if SPDK interrupt mode is globally enabled.
    pub fn interrupt_mode_is_enabled() -> bool {
        unsafe { spdk_interrupt_mode_is_enabled() }
    }

    /// TODO
    #[inline]
    pub fn set_current(&self) {
        unsafe { spdk_set_thread(self.as_ptr()) };
    }

    /// TODO
    #[inline]
    pub fn unset_current(&self) {
        unsafe { spdk_set_thread(std::ptr::null_mut()) };
    }

    /// TODO
    #[inline]
    pub fn is_exited(&self) -> bool {
        unsafe { spdk_thread_is_exited(self.as_ptr()) }
    }

    /// TODO
    ///
    /// # Note
    ///
    /// Avoid any blocking calls as it will block the whole reactor. Also, avoid
    /// long-running functions. In general if you follow the nodejs event loop
    /// model, you should be good.
    pub fn with<T, F: FnOnce() -> T>(self, f: F) -> T {
        let _g = CurrentThreadGuard::new();
        self.set_current();
        f()
    }

    /// TODO
    pub unsafe fn send_msg_unsafe(&self, f: extern "C" fn(ctx: *mut c_void), arg: *mut c_void) {
        let rc = spdk_thread_send_msg(self.as_ptr(), Some(f), arg);
        assert_eq!(rc, 0);
    }

    /// Sends the given thread 'msg' in xPDK speak.
    pub fn send_msg<F, T>(&self, args: T, f: F)
    where
        F: FnOnce(T),
        T: Send,
    {
        // context structure which is passed to the callback as argument
        struct Ctx<F, T> {
            closure: F,
            args: T,
        }

        // helper routine to unpack the closure and its arguments
        extern "C" fn trampoline<F, T>(arg: *mut c_void)
        where
            F: FnOnce(T),
            T: Send,
        {
            let ctx = unsafe { Box::from_raw(arg as *mut Ctx<F, T>) };
            (ctx.closure)(ctx.args);
        }

        let ctx = Box::new(Ctx { closure: f, args });

        let rc = unsafe {
            spdk_thread_send_msg(
                self.as_ptr(),
                Some(trampoline::<F, T>),
                Box::into_raw(ctx).cast(),
            )
        };
        assert_eq!(rc, 0);
    }

    /// Spawns a thread and setting its affinity to the inverse cpu set of
    /// mayastor.
    pub fn spawn_unaffinitized<F, T>(f: F) -> std::thread::JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        std::thread::spawn(|| {
            Self::unaffinitize();
            f()
        })
    }

    /// TODO
    pub fn unaffinitize() {
        Self::unaffinitize_tid(0);

        unsafe {
            trace!("pthread started on core {}", libc::sched_getcpu());
        }
    }

    /// Captures the process's baseline CPU affinity into [`BASE_CPUSET`].
    ///
    /// Must be called once at startup from the main thread, *before* reactors
    /// pin themselves to individual cores. This snapshot is the container's
    /// cgroup cpuset and is used as the seed for off-reactor worker placement.
    pub fn capture_base_cpuset() {
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) == 0 {
                *BASE_CPUSET.lock().unwrap() = Some(set);
            } else {
                trace!("failed to capture base cpuset for worker unaffinitization");
            }
        }
    }

    /// Refreshes [`BASE_CPUSET`] with the process's *current* cgroup cpuset.
    ///
    /// Unlike [`Thread::capture_base_cpuset`], this is safe to call at any
    /// time -- notably from the cpuset monitor's reconcile path after kubelet
    /// has grown or shrunk the container cpuset. It cannot read the cpuset from
    /// the calling thread, because by then reactor and worker threads have
    /// pinned themselves to a subset of the cpuset. Instead it asks the kernel
    /// on a throwaway scratch thread: set the scratch thread's *requested*
    /// affinity to every possible CPU, then read it back. The kernel returns
    /// `all_cpus & cgroup_cpuset`, which is exactly the current cpuset.
    ///
    /// This is race-free with respect to file contents -- a single atomic
    /// kernel view, no `/proc` or cgroupfs parsing (whose files can momentarily
    /// disagree with each other) -- and it tracks both grow and shrink, because
    /// the wide requested mask is re-asserted on the scratch thread every call
    /// (so it does not depend on the kernel restoring a previously-narrowed
    /// mask).
    ///
    /// Returns `true` if the cpuset it read differs from the previously stored
    /// one (so the caller can skip re-applying affinity when nothing changed),
    /// `false` if unchanged or the query failed.
    ///
    /// Note for callers driven by the kubelet `cpu_manager_state` file: kubelet
    /// writes that file *before* it widens the container's cgroup cpuset on a
    /// grow, so a single call right after the file event can still observe the
    /// pre-grow cpuset (returning `false`); the caller should re-poll for a
    /// short window until it changes.
    pub fn refresh_base_cpuset() -> bool {
        let cpuset = std::thread::spawn(|| unsafe {
            let mut all: libc::cpu_set_t = std::mem::zeroed();
            let n = libc::sysconf(libc::_SC_NPROCESSORS_CONF);
            let n = if n > 0 {
                (n as usize).min(libc::CPU_SETSIZE as usize)
            } else {
                libc::CPU_SETSIZE as usize
            };
            for i in 0..n {
                libc::CPU_SET(i, &mut all);
            }
            // Widen the scratch thread's requested mask to all CPUs; the kernel
            // intersects it with the container cpuset.
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &all);

            let mut cur: libc::cpu_set_t = std::mem::zeroed();
            if libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut cur) == 0 {
                Some(cur)
            } else {
                None
            }
        })
        .join()
        .ok()
        .flatten();

        match cpuset {
            Some(set) => {
                let mut guard = BASE_CPUSET.lock().unwrap();
                let changed = match *guard {
                    Some(old) => !unsafe { libc::CPU_EQUAL(&old, &set) },
                    None => true,
                };
                *guard = Some(set);
                changed
            }
            None => false,
        }
    }

    pub fn unaffinitize_tid(tid: libc::pid_t) {
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();

            // Seed the mask from the process's baseline cpuset (the container's
            // cpuset.cpus), captured at startup before reactors pinned
            // themselves. This keeps workers inside the CPUs assigned to this
            // workload -- so restoring affinity can't place them on CPUs
            // reserved for other workloads (other Guaranteed pods'
            // dedicatedCpus, foreign isolcpus) -- while still covering every
            // non-reactor core the workload owns.
            //
            // We must NOT seed from the calling thread's *current* affinity:
            // threads spawned from a reactor context inherit that reactor's
            // single-core pin, so seeding from it and then clearing the reactor
            // cores would leave an empty mask and pin the worker right back onto
            // a reactor core -- the opposite of unaffinitization.
            if let Some(base) = *BASE_CPUSET.lock().unwrap() {
                set = base;
            } else if libc::sched_getaffinity(tid, std::mem::size_of::<libc::cpu_set_t>(), &mut set)
                != 0
            {
                // Fallback (base cpuset was never captured): all online CPUs.
                for i in 0..libc::sysconf(libc::_SC_NPROCESSORS_ONLN) {
                    libc::CPU_SET(i as usize, &mut set);
                }
            }

            // Keep workers off our reactor cores.
            Cores::count().into_iter().for_each(|i| {
                libc::CPU_CLR(i as usize, &mut set);
            });

            // Never pin to an empty set. This only happens when the workload's
            // entire cpuset is reactor cores, so there is genuinely no
            // off-reactor CPU to use: fall back to the whole allowed set
            // (base cpuset if we have it, else the thread's current mask).
            if libc::CPU_COUNT(&set) == 0 {
                if let Some(base) = *BASE_CPUSET.lock().unwrap() {
                    set = base;
                } else {
                    libc::sched_getaffinity(tid, std::mem::size_of::<libc::cpu_set_t>(), &mut set);
                }
            }

            libc::sched_setaffinity(tid, std::mem::size_of::<libc::cpu_set_t>(), &set);
        }
    }

    /// TODO
    pub fn is_spdk_thread() -> bool {
        let thread = unsafe { spdk_get_thread() };
        return !thread.is_null();
    }

    /// TODO
    pub fn from_ptr(ptr: *mut spdk_thread) -> Self {
        Self {
            inner: NonNull::new(ptr).unwrap(),
        }
    }

    /// Returns a pointer to the underlying `spdk_thread` structure.
    pub fn as_ptr(&self) -> *mut spdk_thread {
        self.inner.as_ptr()
    }

    /// Returns string representation of current thread name and core Id.
    pub fn current_info() -> String {
        match Thread::current() {
            Some(t) => {
                format!("{:?}", t)
            }
            None => {
                format!("Non-SPDK thread [core {}]", Cores::current())
            }
        }
    }

    /// Get the interrupt fd_group for this thread.
    ///
    /// Returns a raw pointer to the thread's `spdk_fd_group`, which can
    /// be nested into a reactor-level fd_group for hierarchical event
    /// multiplexing. Only meaningful when interrupt mode is enabled.
    pub fn get_interrupt_fd_group(&self) -> *mut spdk_fd_group {
        unsafe { spdk_thread_get_interrupt_fd_group(self.as_ptr()) }
    }
}

/// Wrapper for `spdk_fd_group` -- an event multiplexing group that
/// aggregates file descriptors and supports hierarchical nesting.
///
/// Used by reactors to block until any nested thread has events,
/// implementing SPDK's interrupt-driven reactor pattern.
pub struct FdGroup {
    inner: NonNull<spdk_fd_group>,
}

impl Debug for FdGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FdGroup({:p})", self.inner)
    }
}

unsafe impl Send for FdGroup {}

impl FdGroup {
    /// Create a new fd_group.
    pub fn create() -> Result<Self, Errno> {
        let mut ptr: *mut spdk_fd_group = std::ptr::null_mut();
        let rc = unsafe { spdk_fd_group_create(&mut ptr) };
        if rc != 0 {
            return Err(Errno::from_raw(rc.abs()));
        }
        Ok(Self {
            inner: NonNull::new(ptr).expect("spdk_fd_group_create returned null"),
        })
    }

    /// Add a file descriptor to this fd_group with a callback.
    ///
    /// When `efd` becomes readable, `fn_` is called with `arg`.
    pub fn add(
        &self,
        efd: i32,
        fn_: unsafe extern "C" fn(*mut c_void) -> i32,
        arg: *mut c_void,
    ) -> Result<(), Errno> {
        let rc = unsafe { spdk_fd_group_add(self.as_ptr(), efd, Some(fn_), arg, std::ptr::null()) };
        if rc != 0 {
            Err(Errno::from_raw(rc.abs()))
        } else {
            Ok(())
        }
    }

    /// Add a file descriptor with an explicit `fd_type`.
    ///
    /// Use `FD_TYPE_EVENTFD` for eventfds so that `fd_group_wait()`
    /// automatically drains them (reads the counter to 0) before
    /// calling the callback. Without this, level-triggered epoll
    /// returns the fd on every call, causing a busy-spin.
    pub fn add_with_fd_type(
        &self,
        efd: i32,
        fn_: unsafe extern "C" fn(*mut c_void) -> i32,
        arg: *mut c_void,
        fd_type: u32,
    ) -> Result<(), Errno> {
        let mut opts: spdk_event_handler_opts = unsafe { std::mem::zeroed() };
        unsafe {
            spdk_fd_group_get_default_event_handler_opts(
                &mut opts,
                std::mem::size_of::<spdk_event_handler_opts>() as u64,
            );
        }
        opts.fd_type = fd_type;
        let rc = unsafe {
            spdk_fd_group_add_ext(
                self.as_ptr(),
                efd,
                Some(fn_),
                arg,
                std::ptr::null(),
                &mut opts,
            )
        };
        if rc != 0 {
            Err(Errno::from_raw(rc.abs()))
        } else {
            Ok(())
        }
    }

    /// Wait for events on the fd_group.
    ///
    /// `timeout` is in milliseconds. -1 blocks forever, 0 is non-blocking.
    /// Returns the number of events processed on success, or a negative
    /// `-errno` on failure. Kept as raw `i32` (not `Result<_, Errno>`)
    /// because the non-error path carries a count, not unit.
    pub fn wait(&self, timeout: i32) -> i32 {
        unsafe { spdk_fd_group_wait(self.as_ptr(), timeout) }
    }

    /// Nest a child fd_group (typically a thread's fd_group) into this
    /// parent fd_group. Events on the child will wake the parent's wait.
    pub fn nest(&self, child: *mut spdk_fd_group) -> Result<(), Errno> {
        let rc = unsafe { spdk_fd_group_nest(self.as_ptr(), child) };
        if rc != 0 {
            Err(Errno::from_raw(rc.abs()))
        } else {
            Ok(())
        }
    }

    /// Remove a previously nested child fd_group.
    pub fn unnest(&self, child: *mut spdk_fd_group) -> Result<(), Errno> {
        let rc = unsafe { spdk_fd_group_unnest(self.as_ptr(), child) };
        if rc != 0 {
            Err(Errno::from_raw(rc.abs()))
        } else {
            Ok(())
        }
    }

    /// Returns the raw pointer to the underlying `spdk_fd_group`.
    pub fn as_ptr(&self) -> *mut spdk_fd_group {
        self.inner.as_ptr()
    }
}

impl Drop for FdGroup {
    fn drop(&mut self) {
        unsafe { spdk_fd_group_destroy(self.as_ptr()) };
    }
}

/// RAII guard for saving and restoring current SPDK thread.
pub struct CurrentThreadGuard {
    previous: Option<Thread>,
}

impl Drop for CurrentThreadGuard {
    fn drop(&mut self) {
        if let Some(t) = self.previous.take() {
            t.set_current();
        }
    }
}

impl CurrentThreadGuard {
    pub fn new() -> Self {
        Self {
            previous: Thread::current(),
        }
    }
}
