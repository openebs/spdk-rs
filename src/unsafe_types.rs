use std::cell::UnsafeCell;

/// Unsafe reference wrapper with provides interior mutability.
///
/// `UnsafeRef<T>` implements `Send` and `Default`, and designed to use a
/// normal Rust reference in contexts requiring these traits. The user must
/// ensure the code validity.
///
/// `Default` implementation leaves `UnsafeRef<T>` in an uninitialized state.
/// The user must initialize it with `replace_ref()` or `replace_mut` before
/// use.
///
/// `UnsafeRef<T>` erases the lifetime and immutability of the underlying
/// reference.
pub struct UnsafeRef<T> {
    ptr: UnsafeCell<*mut T>,
}

unsafe impl<T> Send for UnsafeRef<T> {}

impl<T> Default for UnsafeRef<T> {
    fn default() -> Self {
        Self {
            ptr: UnsafeCell::new(std::ptr::null_mut()),
        }
    }
}

impl<T> UnsafeRef<T> {
    /// Constructs a new instance of `UnsafeRef<T>` which will wrap the
    /// specified immutable reference.
    #[inline(always)]
    pub unsafe fn from_ref(t: &T) -> UnsafeRef<T> {
        Self {
            ptr: UnsafeCell::new(t as *const T as *mut T),
        }
    }

    /// Constructs a new instance of `UnsafeRef<T>` which will wrap the
    /// specified mutable reference.
    #[inline(always)]
    pub fn new(t: &mut T) -> UnsafeRef<T> {
        Self {
            ptr: UnsafeCell::new(t as *mut T),
        }
    }

    /// Replaces the reference in the `UnsafeRef<T>` instance by the
    /// immutable reference given in parameter, returning the old reference if
    /// present.
    #[inline(always)]
    pub unsafe fn replace_ref(&self, t: &T) -> Option<&mut T> {
        self.replace_ptr(t as *const T as *mut T)
    }

    /// Replaces the reference in the `UnsafeRef<T>` instance by the
    /// mutable reference given in parameter, returning the old reference if
    /// present.
    #[inline(always)]
    pub unsafe fn replace_mut(&self, t: &mut T) -> Option<&mut T> {
        self.replace_ptr(t as *mut T)
    }

    /// Moves the reference out of the `UnsafeRef<T>`, leaving it
    /// uninitialized.
    #[inline(always)]
    pub unsafe fn take_mut(&self) -> Option<&mut T> {
        self.replace_ptr(std::ptr::null_mut())
    }

    #[inline(always)]
    unsafe fn replace_ptr(&self, ptr: *mut T) -> Option<&mut T> {
        let prev = *self.ptr.get();
        *(&mut *self.ptr.get()) = ptr;
        prev.as_mut()
    }

    /// Returns `true` if the `UnsafeRef<T>` instance has an initilized
    /// reference, `false` if not.
    #[inline(always)]
    pub fn has_value(&self) -> bool {
        !unsafe { *self.ptr.get() }.is_null()
    }

    /// Returns a mutable pointer to the underlying object.
    #[inline(always)]
    pub unsafe fn as_ptr(&self) -> *mut T {
        *self.ptr.get()
    }

    /// Returns a reference to the underlying object.
    #[inline(always)]
    pub unsafe fn as_ref(&self) -> &T {
        (&*self.ptr.get())
            .as_ref()
            .expect("UnsafeRef must be initialized")
    }

    /// Returns a mutable reference to the underlying object.
    #[inline(always)]
    pub unsafe fn as_mut(&self) -> &mut T {
        (&mut *self.ptr.get())
            .as_mut()
            .expect("UnsafeRef must be initialized")
    }
}

/// Unsafe wrapper with provides interior mutability.
///
/// `UnsafeData<T>` implements `Send` and `Default`, and designed to use an
/// object in contexts requiring these traits. The user must ensure the code
/// validity.
///
/// `Default` implementation leaves `UnsafeData<T>` in an uninitialized state.
/// The user must initialize it with `replace()` before use.
///
/// To wrap a reference, and erase its lifetime, use `UnsafeRef<T>`.
pub struct UnsafeData<T> {
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T> Send for UnsafeData<T> {}

impl<T> Default for UnsafeData<T> {
    fn default() -> Self {
        Self {
            value: UnsafeCell::new(None),
        }
    }
}

impl<T> UnsafeData<T> {
    /// Constructs a new instance of `UnsafeData<T>` which will wrap the
    /// specified object.
    #[inline(always)]
    pub fn new(t: T) -> UnsafeData<T> {
        Self {
            value: UnsafeCell::new(Some(t)),
        }
    }

    /// Replaces the actual value in the `UnsafeData<T>` instance by the
    /// value given in parameter, returning the old value if present.
    #[inline(always)]
    pub unsafe fn replace(&self, t: T) -> Option<T> {
        (&mut *self.value.get()).replace(t)
    }

    /// Moves the value out of the `UnsafeData<T>`, leaving it uninitialized.
    #[inline(always)]
    pub unsafe fn take(&self) -> Option<T> {
        (&mut *self.value.get()).take()
    }

    /// Returns `true` if the `UnsafeRef<T>` instance has an initilized value,
    /// `false` if not.
    #[inline(always)]
    pub fn has_value(&self) -> bool {
        unsafe { &*self.value.get() }.is_some()
    }

    /// Returns a reference to the underlying object.
    #[inline(always)]
    pub unsafe fn as_ref(&self) -> &T {
        (&*self.value.get())
            .as_ref()
            .expect("UnsafeData must be initialized")
    }

    /// Returns a mutable reference to the underlying object.
    #[inline(always)]
    pub unsafe fn as_mut(&self) -> &mut T {
        (&mut *self.value.get())
            .as_mut()
            .expect("UnsafeData must be initialized")
    }
}
