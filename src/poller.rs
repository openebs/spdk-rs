use std::{
    ffi::{c_void, CString},
    fmt,
    os::raw::c_char,
    ptr::NonNull,
    time::Duration,
};

use parking_lot::ReentrantMutex;

use crate::{
    cpu_cores::Cores,
    ffihelper::{AsStr, IntoCString},
    libspdk::{
        spdk_poller,
        spdk_poller_fn,
        spdk_poller_pause,
        spdk_poller_register,
        spdk_poller_register_named,
        spdk_poller_resume,
        spdk_poller_unregister,
    },
    Thread,
};

/// Poller state.
#[derive(Debug, PartialEq)]
enum PollerState {
    Starting,
    Stopped,
    Waiting,
    Running,
}

struct PollerContext(*mut c_void);

unsafe impl Send for PollerContext {}

/// A structure for poller context.
struct PollerInner<'a, T>
where
    T: 'a + Default + Send,
{
    inner_ptr: *mut spdk_poller,
    state: PollerState,
    name: Option<String>,
    interval: u64,
    data: T,
    poll_fn: Box<dyn FnMut(&T) -> i32 + 'a>,
    thread: Option<Thread>,
    lock: ReentrantMutex<()>,
}

unsafe impl<'a, T> Send for PollerInner<'a, T> where T: 'a + Default + Send {}

impl<'a, T> fmt::Debug for PollerInner<'a, T>
where
    T: 'a + Default + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Poller")
            .field("name", &self.dbg_name())
            .field("state", &self.state)
            .field("interval_us", &self.interval)
            .field(
                "thread",
                &self.thread.map_or_else(
                    || "<none>".to_string(),
                    |t| t.name().to_string(),
                ),
            )
            .finish()
    }
}

impl<'a, T> PollerInner<'a, T>
where
    T: 'a + Default + Send,
{
    fn as_ctx(&self) -> PollerContext {
        PollerContext(self as *const Self as *mut Self as *mut c_void)
    }

    fn from_ctx<'b>(p: PollerContext) -> &'b mut Self {
        unsafe { &mut *(p.0 as *mut Self) }
    }

    fn dbg_name(&self) -> &str {
        match &self.name {
            Some(s) => s.as_str(),
            None => "<unnamed>",
        }
    }

    fn is_active(&self) -> bool {
        matches!(self.state, PollerState::Running | PollerState::Waiting)
    }

    fn register(&mut self) {
        assert_eq!(self.state, PollerState::Starting);

        if let Some(t) = self.thread {
            info!(
                "Created an SPDK thread '{}' ({:p}) for poller '{}'",
                t.name(),
                t.as_ptr(),
                self.dbg_name()
            );

            // Register the poller on its own thread.
            t.send_msg(self.as_ctx(), |ctx| {
                let mut p = Self::from_ctx(ctx);
                p.register_impl();
            });
        } else {
            self.register_impl();
        }
    }

    fn register_impl(&mut self) {
        info!(
            "Registering new poller '{}' on {}",
            self.dbg_name(),
            Thread::current_info(),
        );

        let poll_fn: spdk_poller_fn = Some(inner_poller_cb::<T>);

        self.inner_ptr = match &self.name {
            Some(name) => {
                // SPDK stores the name internally.
                let name_ptr = name.as_str().into_cstring();
                unsafe {
                    spdk_poller_register_named(
                        poll_fn,
                        self.as_ctx().0,
                        self.interval,
                        name_ptr.as_ptr(),
                    )
                }
            }
            None => unsafe {
                spdk_poller_register(poll_fn, self.as_ctx().0, self.interval)
            },
        };
        self.state = PollerState::Waiting;
    }

    fn stop(&mut self) {
        info!(
            "Stopping poller '{}' on {}",
            self.dbg_name(),
            Thread::current_info()
        );

        let _g = self.lock.lock();

        assert_ne!(self.state, PollerState::Stopped);

        self.state = PollerState::Stopped;
    }

    fn unregister(&mut self) {
        info!(
            "Unregistering poller '{}' on {}",
            self.dbg_name(),
            Thread::current_info()
        );

        assert_eq!(self.state, PollerState::Stopped);

        if !self.inner_ptr.is_null() {
            unsafe {
                spdk_poller_unregister(&mut self.inner_ptr);
            }
        }

        if let Some(t) = self.thread.take() {
            info!(
                "Exiting poller thread '{}' ({:p}): '{}' ({:p})",
                self.dbg_name(),
                self,
                t.name(),
                t.as_ptr(),
            );
            t.exit();
        }

        unsafe {
            drop(Box::from_raw(self));
        }
    }
}

/// Poller callback.
unsafe extern "C" fn inner_poller_cb<'a, T>(ctx: *mut c_void) -> i32
where
    T: 'a + Default + Send,
{
    let p = PollerInner::<T>::from_ctx(PollerContext(ctx));
    let g = p.lock.lock();

    match p.state {
        PollerState::Waiting => {
            p.state = PollerState::Running;
            (p.poll_fn)(&p.data);
        }
        PollerState::Stopped => {
            drop(g);
            p.unregister();
            return 0;
        }
        _ => {
            panic!("Unexpected poller state before polling: {:?}", p);
        }
    }

    match p.state {
        PollerState::Running => {
            p.state = PollerState::Waiting;
        }
        PollerState::Stopped => {
            drop(g);
            p.unregister();
            return 0;
        }
        _ => {
            panic!("Unexpected poller state after polling: {:?}", p);
        }
    }

    0
}

/// Poller structure that allows us to pause, stop, resume periodic tasks.
///
/// # Generic Arguments
///
/// * `T`: user-defined poller data.
pub struct Poller<'a, T = ()>
where
    T: 'a + Default + Send,
{
    inner: Option<Box<PollerInner<'a, T>>>,
}

impl<'a, T> fmt::Debug for Poller<'a, T>
where
    T: 'a + Default + Send,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner().fmt(f)
    }
}

impl<'a, T> Poller<'a, T>
where
    T: 'a + Default + Send,
{
    /// Consumers the poller instance and stops it. Essentially the same as
    /// dropping the poller.
    pub fn stop(self) {
        drop(self);
    }

    /// Pauses the poller.
    pub fn pause(&self) {
        assert!(self.inner().is_active());

        unsafe {
            spdk_poller_pause(self.inner().inner_ptr);
        }
    }

    /// Resumes the poller.
    pub fn resume(&self) {
        assert!(self.inner().is_active());

        unsafe {
            spdk_poller_resume(self.inner().inner_ptr);
        }
    }

    /// Returns a reference to the poller's data object.
    pub fn data(&self) -> &T {
        &self.inner().data
    }

    /// Returns poller name.
    pub fn name(&self) -> Option<&str> {
        self.inner().name.as_ref().map(|s| s.as_str())
    }

    fn inner(&self) -> &PollerInner<'a, T> {
        self.inner.as_ref().unwrap()
    }
}

impl<'a, T> Drop for Poller<'a, T>
where
    T: 'a + Default + Send,
{
    fn drop(&mut self) {
        let p = self.inner.take().unwrap();
        Box::leak(p).stop();
    }
}

/// Builder type to create a new poller.
pub struct PollerBuilder<'a, T>
where
    T: 'a + Default + Send,
{
    name: Option<String>,
    data: Option<T>,
    poll_fn: Option<Box<dyn FnMut(&T) -> i32 + 'a>>,
    interval: std::time::Duration,
    core: Option<u32>,
}

impl<'a, T> Default for PollerBuilder<'a, T>
where
    T: 'a + Default + Send,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> PollerBuilder<'a, T>
where
    T: 'a + Default + Send,
{
    /// Creates a new nameless poller that runs every time the thread the poller
    /// is created on is polled.
    pub fn new() -> Self {
        Self {
            name: None,
            data: None,
            poll_fn: None,
            interval: Duration::from_micros(0),
            core: None,
        }
    }

    /// Sets optional poller name.
    pub fn with_name(mut self, name: &str) -> Self {
        self.name = Some(String::from(name));
        self
    }

    /// Sets the poller data instance.
    /// This Poller parameter is manadory.
    pub fn with_data(mut self, data: T) -> Self {
        self.data = Some(data);
        self
    }

    /// Sets the poll function for this poller.
    /// This Poller parameter is manadory.
    pub fn with_poll_fn(mut self, poll_fn: impl FnMut(&T) -> i32 + 'a) -> Self {
        self.poll_fn = Some(Box::new(poll_fn));
        self
    }

    /// Sets the polling interval for this poller.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the CPU core to run poller on.
    pub fn with_core(mut self, core: u32) -> Self {
        self.core = Some(core);
        self
    }

    /// Makes a thread name for this poller.
    fn thread_name(&self) -> String {
        match &self.name {
            Some(n) => format!("poller_thread_{}", n),
            None => "poller_thread_unnamed".to_string(),
        }
    }

    /// Consumes a `PollerBuilder` instance, and registers a new poller within
    /// SPDK.
    pub fn build(self) -> Poller<'a, T> {
        // If this poller is configured to run on a different core,
        // create a thread for it.
        let thread = self
            .core
            .map(|core| Thread::new(self.thread_name(), core).unwrap());

        // Create a new poller.
        let mut ctx = Box::new(PollerInner {
            inner_ptr: std::ptr::null_mut(),
            state: PollerState::Starting,
            name: self.name.clone(),
            data: self.data.unwrap_or_default(),
            poll_fn: self.poll_fn.expect("Poller function must be set"),
            interval: self.interval.as_micros() as u64,
            thread,
            lock: ReentrantMutex::new(()),
        });

        ctx.register();

        debug!("New poller context '{}' ({:p})", ctx.dbg_name(), ctx);

        Poller {
            inner: Some(ctx),
        }
    }
}
