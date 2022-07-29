///! TODO
use crate::cpu_cores::Cores;
use crate::{
    cpu_cores::CpuMask,
    libspdk::{
        spdk_get_thread,
        spdk_set_thread,
        spdk_thread,
        spdk_thread_create,
        spdk_thread_destroy,
        spdk_thread_exit,
        spdk_thread_get_by_id,
        spdk_thread_get_id,
        spdk_thread_get_name,
        spdk_thread_is_exited,
        spdk_thread_poll,
        spdk_thread_send_msg,
    },
};
use futures::channel::oneshot::{channel, Receiver, Sender};
use nix::{errno::Errno, libc};
use std::{
    ffi::{c_void, CStr, CString},
    fmt::Debug,
    future::Future,
    ptr::NonNull,
};

/// Wrapper for `spdk_thread`.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Thread {
    inner: NonNull<spdk_thread>,
}

unsafe impl Send for Thread {}

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
        .map(|inner| Self {
            inner,
        })
    }

    /// Destroys the given thread waiting for it to become ready to destroy.
    pub fn destroy(self) {
        debug!(
            "Destroying SPDK thread {}... {:p}",
            self.name(),
            self.as_ptr()
        );

        unsafe {
            spdk_set_thread(self.as_ptr());

            // set that we *want* to exit, but we have not exited yet
            spdk_thread_exit(self.as_ptr());

            // now wait until the thread is actually exited the internal
            // state is updated by spdk_thread_poll()
            while !spdk_thread_is_exited(self.as_ptr()) {
                spdk_thread_poll(self.as_ptr(), 0, 0);
            }
            spdk_thread_destroy(self.as_ptr());
        }

        debug!("SPDK thread {:p} destroyed", self.as_ptr());
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

    /// Returns thread indetifier.
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

    /// TODO
    #[inline]
    pub fn enter(&self) {
        unsafe { spdk_set_thread(self.as_ptr()) };
    }

    /// TODO
    #[inline]
    pub fn exit(&self) {
        unsafe { spdk_set_thread(std::ptr::null_mut()) };
    }

    /// TODO
    ///
    /// # Note
    ///
    /// Avoid any blocking calls as it will block the whole reactor. Also, avoid
    /// long-running functions. In general if you follow the nodejs event loop
    /// model, you should be good.
    pub fn with<T, F: FnOnce() -> T>(self, f: F) -> T {
        let th = Self::current();
        self.enter();
        let out = f();
        if let Some(t) = th {
            t.enter();
        }
        out
    }

    /// TODO
    pub unsafe fn send_msg_unsafe(
        &self,
        f: extern "C" fn(ctx: *mut c_void),
        arg: *mut c_void,
    ) {
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

        let ctx = Box::new(Ctx {
            closure: f,
            args,
        });

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
        unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            for i in 0 .. libc::sysconf(libc::_SC_NPROCESSORS_ONLN) {
                libc::CPU_SET(i as usize, &mut set)
            }

            Cores::count()
                .into_iter()
                .for_each(|i| libc::CPU_CLR(i as usize, &mut set));

            libc::sched_setaffinity(
                0,
                std::mem::size_of::<libc::cpu_set_t>(),
                &set,
            );

            debug!("pthread started on core {}", libc::sched_getcpu());
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
}
